// Runs Metal canvas contracts on AppKit's main thread.
use std::cell::RefCell;
use std::rc::Rc;

use zpd_appkit::geometry::{Point, Rect};
use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AsView, CanvasFrame, CanvasInput, CanvasInputDelegate, MetalCanvasDelegate, MetalCanvasView,
};

/// Records each render's layer and frame.
#[derive(Default)]
struct Probe {
    renders: RefCell<Vec<(usize, CanvasFrame)>>,
}

impl CanvasInputDelegate for Probe {
    fn input(&self, _input: &CanvasInput) -> bool {
        false
    }

    fn title_bar_hit(&self, location: Point) -> bool {
        location.x < 100.0
    }
}

impl MetalCanvasDelegate for Probe {
    fn render(&self, layer: zpd_appkit::ui::MetalLayer<'_>, frame: CanvasFrame) {
        self.renders
            .borrow_mut()
            .push((layer.as_ptr() as usize, frame));
    }
}

struct MetalChecks;

impl MetalChecks {
    /// Returns the canvas, the only subview of the only window this test creates.
    fn view() -> zpd_objc::Id {
        // SAFETY: This isolated main-thread test creates one NSWindow with one subview.
        unsafe {
            let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => zpd_objc::Id);
            let windows = zpd_objc::msg_send!(app, zpd_objc::sel!("windows"), () => zpd_objc::Id);
            let window = zpd_objc::msg_send!(windows, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => zpd_objc::Id);
            let content = zpd_objc::msg_send!(window, zpd_objc::sel!("contentView"), () => zpd_objc::Id);
            let subviews = zpd_objc::msg_send!(content, zpd_objc::sel!("subviews"), () => zpd_objc::Id);
            zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => zpd_objc::Id)
        }
    }

    /// Returns the class name of the canvas's backing layer.
    fn layer_class() -> String {
        // SAFETY: The canvas is live; the class description is an autoreleased string.
        unsafe {
            let layer = zpd_objc::msg_send!(Self::view(), zpd_objc::sel!("layer"), () => zpd_objc::Id);
            let class = zpd_objc::msg_send!(layer, zpd_objc::sel!("class"), () => zpd_objc::Id);
            let name = zpd_objc::msg_send!(class, zpd_objc::sel!("description"), () => zpd_objc::Id);
            let utf8 = zpd_objc::msg_send!(name, zpd_objc::sel!("UTF8String"), () => *const std::ffi::c_char);
            std::ffi::CStr::from_ptr(utf8).to_string_lossy().into_owned()
        }
    }

    /// Returns the backing layer's `contentsScale`.
    fn contents_scale() -> f64 {
        // SAFETY: The canvas is live and layer-backed.
        unsafe {
            let layer = zpd_objc::msg_send!(Self::view(), zpd_objc::sel!("layer"), () => zpd_objc::Id);
            zpd_objc::msg_send!(layer, zpd_objc::sel!("contentsScale"), () => f64)
        }
    }

    /// Runs the main run loop briefly so queued renders happen.
    fn run_loop() {
        // SAFETY: The test runs on the main thread, which owns the main run loop.
        unsafe {
            let date = zpd_objc::msg_send!(zpd_objc::class!("NSDate"), zpd_objc::sel!("dateWithTimeIntervalSinceNow:"), ((0.05): f64) => zpd_objc::Id);
            let run_loop = zpd_objc::msg_send!(zpd_objc::class!("NSRunLoop"), zpd_objc::sel!("currentRunLoop"), () => zpd_objc::Id);
            zpd_objc::msg_send!(run_loop, zpd_objc::sel!("runUntilDate:"), ((date): zpd_objc::Id) => ());
        }
    }
}

fn main() {
    let application = Application::new().unwrap();
    let window = application.create_window().unwrap();
    window.set_bounds(Rect::new(100.0, 100.0, 400.0, 300.0)).unwrap();
    let content = window.content_view().unwrap();
    let canvas = MetalCanvasView::new(&application).unwrap();
    content.add_subview(&canvas).unwrap();
    canvas
        .as_view()
        .set_frame(Rect::new(0.0, 0.0, 120.0, 80.0))
        .unwrap();

    // Verifies the view is backed by a CAMetalLayer.
    assert_eq!(MetalChecks::layer_class(), "CAMetalLayer");

    // Verifies repeated invalidations queue one render of the layer with the view's
    // bounds and window scale, and none runs synchronously.
    let probe = Rc::new(Probe::default());
    canvas.set_delegate(probe.clone());
    MetalChecks::run_loop();
    probe.renders.borrow_mut().clear();
    canvas.handle().set_needs_display().unwrap();
    canvas.handle().set_needs_display().unwrap();
    assert!(probe.renders.borrow().is_empty());
    MetalChecks::run_loop();
    let renders = probe.renders.borrow().clone();
    assert_eq!(renders.len(), 1);
    let (layer, frame) = renders[0];
    assert_ne!(layer, 0);
    assert_eq!(frame.bounds, Rect::new(0.0, 0.0, 120.0, 80.0));
    assert!(frame.scale >= 1.0);

    // Verifies the layer's contentsScale follows the frame's scale.
    assert_eq!(MetalChecks::contents_scale(), frame.scale);

    // Verifies a resize alone renders again with the new size.
    canvas
        .as_view()
        .set_frame(Rect::new(0.0, 0.0, 200.0, 100.0))
        .unwrap();
    MetalChecks::run_loop();
    let renders = probe.renders.borrow();
    assert_eq!(
        renders.last().unwrap().1.bounds,
        Rect::new(0.0, 0.0, 200.0, 100.0)
    );
}
