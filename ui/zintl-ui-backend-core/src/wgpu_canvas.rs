//! Canvas views that a handler renders with wgpu.
//!
//! They share [`CanvasNode`](crate::CanvasNode), redraw policies, input, and
//! reconciliation with [`Canvas2DHandler`](crate::Canvas2DHandler) canvases;
//! only drawing differs.

use std::any::Any;
use std::error::Error;
use std::fmt;

use zintl_ui_graphics::Point;

use crate::canvas::{ErasedHandler, SetupInputs};
use crate::{CanvasEvent, CanvasEventContext, CanvasNode, Frame, Invalidator, Redraw};

/// Renders a canvas with wgpu and handles its input.
///
/// All methods run on the main thread, which also builds views. `Clone` lets
/// the declaring view be rendered repeatedly.
pub trait WgpuCanvasHandler: Clone + 'static {
    /// Runs once before the first render, when the GPU is available.
    ///
    /// Create pipelines and other GPU resources here. Input may arrive before it.
    fn setup(&mut self, cx: &mut WgpuSetup<'_>) {
        let _ = cx;
    }

    /// Renders one frame into `target`; the backend presents it afterwards.
    fn render(&mut self, target: &mut WgpuTarget<'_>, frame: &Frame);

    /// Handles pointer, keyboard, and focus input; returns whether it was consumed.
    ///
    /// Unconsumed input continues to the platform, for example to a parent
    /// scroll view or the window's key handling.
    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        let _ = (event, cx);
        false
    }

    /// Returns whether a click at `point` belongs to the canvas although it lies
    /// under the window's title bar.
    fn title_bar_hit(&self, point: Point) -> bool {
        let _ = point;
        false
    }

    /// Receives the handler from a later render of the same canvas.
    ///
    /// The default replaces `self`, which drops GPU resources created in
    /// [`WgpuCanvasHandler::setup`]. Override it to keep them.
    fn reconcile(&mut self, next: Self) -> Redraw {
        *self = next;
        Redraw::Yes
    }
}

/// The wgpu objects canvases of one backend share.
#[derive(Clone, Debug)]
pub struct WgpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl WgpuContext {
    /// Requests the default adapter and a device with default limits, blocking until ready.
    pub fn new() -> Result<Self, WgpuError> {
        Self::with_descriptor(wgpu::InstanceDescriptor::new_without_display_handle_from_env())
    }

    /// Like [`WgpuContext::new`], limited to `backends`, such as the only one
    /// whose surfaces a platform's views accept.
    pub fn for_backends(backends: wgpu::Backends) -> Result<Self, WgpuError> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        descriptor.backends = backends;
        Self::with_descriptor(descriptor)
    }

    fn with_descriptor(descriptor: wgpu::InstanceDescriptor) -> Result<Self, WgpuError> {
        let instance = wgpu::Instance::new(descriptor);
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(WgpuError::Adapter)?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(WgpuError::Device)?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

/// Why a [`WgpuContext`] could not be created.
#[derive(Debug)]
pub enum WgpuError {
    Adapter(wgpu::RequestAdapterError),
    Device(wgpu::RequestDeviceError),
    Surface(wgpu::CreateSurfaceError),
}

impl fmt::Display for WgpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Adapter(error) => write!(formatter, "no suitable GPU adapter: {error}"),
            Self::Device(error) => write!(formatter, "GPU device request failed: {error}"),
            Self::Surface(error) => write!(formatter, "GPU surface creation failed: {error}"),
        }
    }
}

impl Error for WgpuError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Adapter(error) => Some(error),
            Self::Device(error) => Some(error),
            Self::Surface(error) => Some(error),
        }
    }
}

/// Context for [`WgpuCanvasHandler::setup`].
pub struct WgpuSetup<'a> {
    gpu: &'a GpuBinding,
    invalidator: Invalidator,
}

impl WgpuSetup<'_> {
    pub fn context(&self) -> &WgpuContext {
        &self.gpu.context
    }

    pub fn device(&self) -> &wgpu::Device {
        self.gpu.context.device()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.gpu.context.queue()
    }

    /// The format of every [`WgpuTarget`] this canvas renders into.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.gpu.format
    }

    /// Returns a handle that requests a redraw from any thread.
    pub fn invalidator(&self) -> Invalidator {
        self.invalidator.clone()
    }
}

/// The texture a [`WgpuCanvasHandler`] renders one frame into.
pub struct WgpuTarget<'a> {
    context: &'a WgpuContext,
    texture: &'a wgpu::Texture,
    view: &'a wgpu::TextureView,
}

impl<'a> WgpuTarget<'a> {
    /// Wraps a texture for a render; backends create one per frame.
    pub fn new(
        context: &'a WgpuContext,
        texture: &'a wgpu::Texture,
        view: &'a wgpu::TextureView,
    ) -> Self {
        Self {
            context,
            texture,
            view,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        self.context.device()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.context.queue()
    }

    pub fn texture(&self) -> &wgpu::Texture {
        self.texture
    }

    /// A view of the whole texture, for a render pass's color attachment.
    pub fn view(&self) -> &wgpu::TextureView {
        self.view
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture.format()
    }

    /// The texture size in device pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.texture.width(), self.texture.height())
    }
}

/// The GPU a mounted wgpu canvas renders with.
pub(crate) struct GpuBinding {
    pub(crate) context: WgpuContext,
    pub(crate) format: wgpu::TextureFormat,
}

/// Adapts a [`WgpuCanvasHandler`] to the erased handler interface.
struct WgpuHandler<H> {
    handler: H,
    /// Whether `setup` ran, so rendering has its resources.
    ready: bool,
}

impl<H: WgpuCanvasHandler> ErasedHandler for WgpuHandler<H> {
    fn setup(&mut self, inputs: SetupInputs<'_>) -> bool {
        let Some(gpu) = inputs.gpu else {
            return false;
        };
        self.handler.setup(&mut WgpuSetup {
            gpu,
            invalidator: inputs.invalidator,
        });
        self.ready = true;
        true
    }

    fn render(&mut self, target: &mut WgpuTarget<'_>, frame: &Frame) {
        if self.ready {
            self.handler.render(target, frame);
        }
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        self.handler.event(event, cx)
    }

    fn title_bar_hit(&self, point: Point) -> bool {
        self.handler.title_bar_hit(point)
    }

    fn reconcile(&mut self, next: Box<dyn Any>) -> Redraw {
        let next = next
            .downcast::<Self>()
            .expect("canvases reconcile only handlers of the same type");
        self.handler.reconcile(next.handler)
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl CanvasNode {
    /// Wraps a handler that renders with wgpu.
    pub fn wgpu<H: WgpuCanvasHandler>(handler: H) -> Self {
        Self::with_handler_box::<H>(Box::new(WgpuHandler {
            handler,
            ready: false,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use zintl_ui_graphics::Rect;

    use super::*;
    use crate::{CanvasHost, Modifiers, PointerEvent, PointerPhase};

    /// Counts setups, renders, and events; `value` stands in for declared state.
    #[derive(Clone)]
    struct Counter {
        value: u32,
        setups: Rc<Cell<u32>>,
        renders: Rc<Cell<u32>>,
        events: Rc<Cell<u32>>,
    }

    impl Counter {
        fn new(value: u32) -> Self {
            Self {
                value,
                setups: Rc::default(),
                renders: Rc::default(),
                events: Rc::default(),
            }
        }
    }

    impl WgpuCanvasHandler for Counter {
        fn setup(&mut self, cx: &mut WgpuSetup<'_>) {
            assert_eq!(cx.format(), wgpu::TextureFormat::Rgba8Unorm);
            self.setups.set(self.setups.get() + 1);
        }

        fn render(&mut self, target: &mut WgpuTarget<'_>, _frame: &Frame) {
            assert_eq!(target.size(), (4, 4));
            self.renders.set(self.renders.get() + 1);
        }

        fn event(&mut self, _event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
            self.events.set(self.events.get() + 1);
            cx.invalidate();
            true
        }

        fn reconcile(&mut self, next: Self) -> Redraw {
            let redraw = if next.value == self.value {
                Redraw::No
            } else {
                Redraw::Yes
            };
            self.value = next.value;
            redraw
        }
    }

    /// Counts invalidations.
    #[derive(Default)]
    struct Host {
        invalidations: Cell<u32>,
    }

    impl CanvasHost for Host {
        fn invalidate(&self) {
            self.invalidations.set(self.invalidations.get() + 1);
        }

        fn set_focus(&self, _focused: bool) {}

        fn set_cursor(&self, _cursor: crate::Cursor) {}

        fn actions_emitted(&self, _canvas: crate::CanvasId) {}
    }

    fn frame() -> Frame {
        Frame {
            bounds: Rect::from_size(2.0, 2.0),
            safe_area: Rect::from_size(2.0, 2.0),
            scale: 2.0,
            time: 0.0,
            delta: 0.0,
        }
    }

    fn pointer() -> CanvasEvent {
        CanvasEvent::Pointer(PointerEvent {
            phase: PointerPhase::Down,
            button: None,
            location: Point::ZERO,
            modifiers: Modifiers::default(),
            click_count: 1,
        })
    }

    #[test]
    fn input_arrives_before_the_gpu_without_setup() {
        // Verifies events reach a wgpu handler before its GPU is attached, and setup waits for the GPU.
        let handler = Counter::new(1);
        let (setups, events) = (handler.setups.clone(), handler.events.clone());
        let mounted = CanvasNode::wgpu(handler);
        assert!(mounted.event(&pointer()));
        assert_eq!((events.get(), setups.get()), (1, 0));
        // A 2D draw request does not reach a wgpu handler.
        assert!(mounted.record(&frame()).is_empty());
    }

    #[test]
    fn adopt_reconciles_wgpu_handlers() {
        // Verifies re-renders reconcile into the mounted wgpu handler and redraw only on change.
        let host = Rc::new(Host::default());
        let handler = Counter::new(1);
        let events = handler.events.clone();
        let mounted = CanvasNode::wgpu(handler);
        mounted.attach(host.clone(), None);

        mounted.adopt(&CanvasNode::wgpu(Counter::new(1)));
        assert_eq!(host.invalidations.get(), 0);
        mounted.adopt(&CanvasNode::wgpu(Counter::new(2)));
        assert_eq!(host.invalidations.get(), 1);
        // The mounted handler keeps its own counters.
        mounted.event(&pointer());
        assert_eq!(events.get(), 1);
    }

    #[test]
    fn render_sets_up_once_with_the_attached_gpu() {
        // Verifies the first render runs setup with the attached format, and later renders do not.
        // Machines without a GPU adapter skip this check.
        let Ok(context) = WgpuContext::new() else {
            return;
        };
        let handler = Counter::new(1);
        let (setups, renders) = (handler.setups.clone(), handler.renders.clone());
        let mounted = CanvasNode::wgpu(handler);
        mounted.attach_gpu(context.clone(), wgpu::TextureFormat::Rgba8Unorm);

        let texture = context.device().create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        for _ in 0..2 {
            mounted.render(&mut WgpuTarget::new(&context, &texture, &view), &frame());
        }
        assert_eq!((setups.get(), renders.get()), (1, 2));
    }
}
