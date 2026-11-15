// Checks that canvases draw with system colors resolved for their appearance.
#[path = "../../src/native.rs"]
mod native;

use std::cell::Cell;
use std::rc::Rc;

use zpd_appkit::geometry::Rect;
use zpd_appkit::graphics::GraphicsContext;
use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AsView, CanvasDelegate, CanvasFrame, CanvasInput, CanvasInputDelegate, CanvasPalette, CanvasView,
};

/// Records the palette of the last frame drawn.
struct Recorder(Rc<Cell<Option<CanvasPalette>>>);

impl CanvasInputDelegate for Recorder {
    fn input(&self, _input: &CanvasInput) -> bool {
        false
    }
}

impl CanvasDelegate for Recorder {
    fn draw(&self, _context: &mut GraphicsContext<'_>, frame: CanvasFrame) {
        self.0.set(Some(frame.palette));
    }
}

struct PaletteChecks;

impl PaletteChecks {
    /// Returns the canvas, the only subview of the only window's content view.
    fn canvas() -> zpd_objc::Id {
        // SAFETY: This isolated main-thread test creates one window holding one canvas.
        unsafe {
            let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => zpd_objc::Id);
            let windows = zpd_objc::msg_send!(app, zpd_objc::sel!("windows"), () => zpd_objc::Id);
            let window = zpd_objc::msg_send!(windows, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => zpd_objc::Id);
            let content = zpd_objc::msg_send!(window, zpd_objc::sel!("contentView"), () => zpd_objc::Id);
            let subviews = zpd_objc::msg_send!(content, zpd_objc::sel!("subviews"), () => zpd_objc::Id);
            zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => zpd_objc::Id)
        }
    }

    /// Gives the canvas the named appearance.
    fn set_appearance(name: &str) {
        let name = native::nsstring(name);
        // SAFETY: The canvas is live; `appearanceNamed:` returns an autoreleased appearance.
        unsafe {
            let canvas = Self::canvas();
            let appearance = zpd_objc::msg_send!(zpd_objc::class!("NSAppearance"), zpd_objc::sel!("appearanceNamed:"), ((name.as_ptr()): zpd_objc::Id) => zpd_objc::Id);
            zpd_objc::msg_send!(canvas, zpd_objc::sel!("setAppearance:"), ((appearance): zpd_objc::Id) => ());
        }
    }

    /// Runs the main run loop briefly, so AppKit applies appearance changes and displays.
    fn run_briefly() {
        // SAFETY: Runs the current thread's run loop; the date is autoreleased.
        unsafe {
            let date = zpd_objc::msg_send!(zpd_objc::class!("NSDate"), zpd_objc::sel!("dateWithTimeIntervalSinceNow:"), ((0.3): f64) => zpd_objc::Id);
            let run_loop = zpd_objc::msg_send!(zpd_objc::class!("NSRunLoop"), zpd_objc::sel!("currentRunLoop"), () => zpd_objc::Id);
            zpd_objc::msg_send!(run_loop, zpd_objc::sel!("runUntilDate:"), ((date): zpd_objc::Id) => ());
        }
    }

    /// Draws the canvas now.
    fn display() {
        // SAFETY: The canvas is live and in a window.
        unsafe { zpd_objc::msg_send!(Self::canvas(), zpd_objc::sel!("display"), () => ()) };
    }
}

fn main() {
    let application = Application::new().unwrap();
    let window = application.create_window().unwrap();
    window.set_bounds(Rect::new(100.0, 100.0, 200.0, 100.0)).unwrap();
    let content = window.content_view().unwrap();
    let canvas = CanvasView::new(&application).unwrap();
    content.add_subview(&canvas).unwrap();
    canvas.as_view().set_frame(content.bounds().unwrap()).unwrap();
    let palette = Rc::new(Cell::new(None));
    canvas.set_delegate(Rc::new(Recorder(palette.clone())));

    PaletteChecks::set_appearance("NSAppearanceNameAqua");
    PaletteChecks::display();
    PaletteChecks::run_briefly();
    let light = palette.take().expect("the canvas draws");
    // Verifies an idle canvas is not drawn again, so the next draw comes from the change.
    PaletteChecks::run_briefly();
    assert!(palette.take().is_none());
    // Verifies an appearance change draws the canvas again without an invalidation.
    PaletteChecks::set_appearance("NSAppearanceNameDarkAqua");
    PaletteChecks::run_briefly();
    let dark = palette
        .take()
        .expect("the canvas draws again for the new appearance");

    // Verifies system colors are resolved for the canvas's appearance, not a fixed value.
    assert_ne!(light.green, dark.green);
    assert_ne!(light.gray, dark.gray);
    // Verifies the colors are the system hues in both appearances.
    for palette in [light, dark] {
        assert!(palette.green.green > palette.green.red && palette.green.green > palette.green.blue);
        assert!(palette.red.red > palette.red.green && palette.red.red > palette.red.blue);
    }
}
