//! Connects core wgpu canvas handlers to external `zpd-winui3` canvases.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;

use zintl_ui_backend_core::graphics::Point;
use zintl_ui_backend_core::tree::ErrorSlot;
use zintl_ui_backend_core::{CanvasNode, MainTaskSender, WgpuContext, WgpuError, WgpuTarget, wgpu};
use zpd_winui3::{CanvasDelegate, CanvasFrame, CanvasInput};

use super::canvas::{Host, InputRoute};
use super::{AppError, GpuSlot};

impl GpuSlot {
    /// Returns the context every canvas of the app shares, creating it on first use.
    ///
    /// Only Direct3D 12 presents to a `SwapChainPanel`. A failure is not kept,
    /// so a later canvas tries again.
    fn context(&self) -> Result<WgpuContext, WgpuError> {
        if let Some(context) = self.context.borrow().as_ref() {
            return Ok(context.clone());
        }
        let created = WgpuContext::for_backends(wgpu::Backends::DX12)?;
        *self.context.borrow_mut() = Some(created.clone());
        Ok(created)
    }
}

/// A surface over the canvas's swap chain panel and its current configuration.
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
    /// The canvas's `ISwapChainPanelNative`, valid while the canvas owns this bridge.
    panel: *mut c_void,
    /// Created at the first render.
    surface: RefCell<Option<Surface>>,
    errors: ErrorSlot<AppError>,
    input: InputRoute,
}

impl WgpuBridge {
    /// Installs `canvas` on `view` and connects its host operations.
    ///
    /// GPU failures are recorded in `errors`; the view then stays blank.
    pub(super) fn install(
        view: &zpd_winui3::Canvas,
        canvas: &CanvasNode,
        input: InputRoute,
        main_tasks: Arc<MainTaskSender>,
        gpu: &GpuSlot,
        errors: &ErrorSlot<AppError>,
    ) -> Result<(), AppError> {
        let context = gpu
            .context()
            .map_err(|error| errors.record(AppError::Wgpu(error)))
            .ok();
        canvas.attach(Rc::new(Host::new(view, &input)?), Some(main_tasks));
        view.set_delegate(Rc::new(Self {
            canvas: canvas.clone(),
            context,
            panel: view.swap_chain_panel(),
            surface: RefCell::new(None),
            errors: errors.clone(),
            input,
        }))?;
        Ok(())
    }

    /// Creates a surface over the panel and connects the canvas to the GPU on first use.
    fn create_surface(&self, context: &WgpuContext) -> Result<Surface, WgpuError> {
        // SAFETY: `panel` is the canvas's live ISwapChainPanelNative, which outlives this
        // bridge and so the surface; wgpu retains it.
        let surface = unsafe {
            context
                .instance()
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::SwapChainPanel(self.panel))
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
        // The panel composes with the content behind it, so premultiplied alpha is preferred.
        let alpha_mode = [
            wgpu::CompositeAlphaMode::PreMultiplied,
            wgpu::CompositeAlphaMode::Opaque,
        ]
        .into_iter()
        .find(|mode| capabilities.alpha_modes.contains(mode))
        .or_else(|| capabilities.alpha_modes.first().copied())
        .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 1,
            height: 1,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
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
    fn acquire(&self, context: &WgpuContext, size: (u32, u32)) -> Option<wgpu::SurfaceTexture> {
        let mut slot = self.surface.borrow_mut();
        if slot.is_none() {
            drop(slot);
            let surface = self
                .create_surface(context)
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
                // The next render creates a new surface over the same panel.
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

impl CanvasDelegate for WgpuBridge {
    fn render(&self, frame: &CanvasFrame) {
        let Some(context) = &self.context else {
            return;
        };
        let size = (
            (frame.width * frame.scale).round() as u32,
            (frame.height * frame.scale).round() as u32,
        );
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        let Some(texture) = self.acquire(context, size) else {
            return;
        };
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.canvas.render(
            &mut WgpuTarget::new(context, &texture.texture, &view),
            &InputRoute::frame(frame),
        );
        context.queue().present(texture);
    }

    fn input(&self, input: &CanvasInput) -> bool {
        self.input.deliver(&self.canvas, input)
    }

    fn title_bar_hit(&self, x: f64, y: f64) -> bool {
        self.canvas.title_bar_hit(Point::new(x, y))
    }
}
