//! Connects core wgpu canvas handlers to `zpd-appkit` Metal canvas views.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use zintl_ui_backend_core::tree::ErrorSlot;
use zintl_ui_backend_core::wgpu;
use zpd_appkit::geometry;
use zpd_appkit::ui::{CanvasFrame, CanvasInput, CanvasInputDelegate, MetalCanvasDelegate};
use zpd_appkit::ui::{MetalCanvasView, MetalLayer};

use super::AppError;
use super::canvas::Host;
use crate::convert::Shared;
use crate::graphics::Point;
use crate::{CanvasNode, MainTaskSender, WgpuContext, WgpuError, WgpuTarget};

/// A surface over the view's `CAMetalLayer` and its current configuration.
struct Surface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// Whether `config` must be applied before the next frame.
    stale: bool,
}

/// Renders and routes input for one mounted wgpu canvas.
pub(super) struct WgpuBridge {
    canvas: CanvasNode,
    /// `None` when no GPU is available; the canvas then only handles input.
    context: Option<WgpuContext>,
    /// Created at the first render, once the layer has its window's scale.
    surface: RefCell<Option<Surface>>,
    errors: ErrorSlot<AppError>,
}

impl WgpuBridge {
    /// Installs `canvas` on `view` and connects its host operations.
    ///
    /// GPU failures are recorded in `errors`; the view then stays blank.
    pub(super) fn install(
        view: &MetalCanvasView,
        canvas: &CanvasNode,
        sender: Arc<MainTaskSender>,
        errors: &ErrorSlot<AppError>,
    ) {
        let context = Self::shared_context()
            .map_err(|error| errors.record(AppError::Wgpu(error)))
            .ok();
        view.set_delegate(Rc::new(Self {
            canvas: canvas.clone(),
            context,
            surface: RefCell::new(None),
            errors: errors.clone(),
        }));
        canvas.attach(Rc::new(Host(view.handle())), Some(sender));
    }

    /// Returns the context every canvas of the main thread shares, creating it once.
    fn shared_context() -> Result<WgpuContext, WgpuError> {
        thread_local! {
            static CONTEXT: RefCell<Option<WgpuContext>> = const { RefCell::new(None) };
        }
        CONTEXT.with(|context| {
            if let Some(context) = context.borrow().as_ref() {
                return Ok(context.clone());
            }
            let created = WgpuContext::new()?;
            *context.borrow_mut() = Some(created.clone());
            Ok(created)
        })
    }

    /// Creates a surface over `layer` and connects the canvas to the GPU on first use.
    fn create_surface(
        &self,
        context: &WgpuContext,
        layer: MetalLayer<'_>,
    ) -> Result<Surface, WgpuError> {
        // SAFETY: `layer` is a live CAMetalLayer; wgpu retains it for the surface's lifetime.
        let surface =
            unsafe {
                context.instance().create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer.as_ptr()),
                )
            }
            .map_err(WgpuError::Surface)?;
        let capabilities = surface.get_capabilities(context.adapter());
        // A recreated surface keeps the format the handler was set up with.
        let format = self
            .surface
            .borrow()
            .as_ref()
            .map(|surface| surface.config.format)
            .or_else(|| {
                capabilities
                    .formats
                    .iter()
                    .copied()
                    .find(wgpu::TextureFormat::is_srgb)
            })
            .or_else(|| capabilities.formats.first().copied())
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 1,
            height: 1,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: Vec::new(),
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        if self.surface.borrow().is_none() {
            self.canvas.attach_gpu(context.clone(), format);
        }
        Ok(Surface {
            surface,
            config,
            stale: true,
        })
    }

    /// Returns the surface's next texture, reconfiguring it for `size` first if needed.
    fn acquire(
        &self,
        context: &WgpuContext,
        layer: MetalLayer<'_>,
        size: (u32, u32),
    ) -> Option<wgpu::SurfaceTexture> {
        let mut slot = self.surface.borrow_mut();
        if slot.is_none() {
            drop(slot);
            let surface = self
                .create_surface(context, layer)
                .map_err(|error| self.errors.record(AppError::Wgpu(error)))
                .ok()?;
            slot = self.surface.borrow_mut();
            *slot = Some(surface);
        }
        let surface = slot.as_mut()?;
        if surface.stale || (surface.config.width, surface.config.height) != size {
            (surface.config.width, surface.config.height) = size;
            surface.surface.configure(context.device(), &surface.config);
            surface.stale = false;
        }
        match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => Some(texture),
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                surface.stale = true;
                Some(texture)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.stale = true;
                self.canvas.invalidate();
                None
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // The next render creates a new surface over the same layer.
                slot.take();
                self.canvas.invalidate();
                None
            }
            // The frame is skipped; later renders try again.
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => None,
        }
    }
}

impl MetalCanvasDelegate for WgpuBridge {
    fn render(&self, layer: MetalLayer<'_>, frame: CanvasFrame) {
        let Some(context) = &self.context else {
            return;
        };
        let size = (
            (frame.bounds.width * frame.scale).round() as u32,
            (frame.bounds.height * frame.scale).round() as u32,
        );
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        let Some(texture) = self.acquire(context, layer, size) else {
            return;
        };
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.canvas.render(
            &mut WgpuTarget::new(context, &texture.texture, &view),
            &frame.shared(),
        );
        context.queue().present(texture);
    }
}

impl CanvasInputDelegate for WgpuBridge {
    fn input(&self, input: &CanvasInput) -> bool {
        self.canvas.event(&input.clone().shared())
    }

    fn title_bar_hit(&self, location: geometry::Point) -> bool {
        self.canvas
            .title_bar_hit(Point::new(location.x, location.y))
    }
}
