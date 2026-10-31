// Runs title bar hit-testing contracts on AppKit's main thread.
#[path = "../../src/native.rs"]
mod native;

use zpd_appkit::geometry::Rect;
use zpd_appkit::runloop::Application;
use std::cell::Cell;
use std::rc::Rc;

use zpd_appkit::geometry::Point;
use zpd_appkit::graphics::GraphicsContext;
use zpd_appkit::ui::{AsView, CanvasDelegate, CanvasFrame, CanvasInput, CanvasView, TitleBar};

/// Claims title bar clicks left of x = 100, like tabs drawn by the canvas.
struct Tabs;

impl CanvasDelegate for Tabs {
    fn draw(&self, _context: &mut GraphicsContext<'_>, _frame: CanvasFrame) {}

    fn input(&self, _input: &CanvasInput) -> bool {
        false
    }

    fn title_bar_hit(&self, location: Point) -> bool {
        location.x < 100.0
    }
}

struct TitleBarChecks;

impl TitleBarChecks {
    /// Returns the only NSWindow this test creates.
    fn window() -> zpd_objc::Id {
        // SAFETY: This isolated main-thread test creates exactly one NSWindow.
        unsafe {
            let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => zpd_objc::Id);
            let windows = zpd_objc::msg_send!(app, zpd_objc::sel!("windows"), () => zpd_objc::Id);
            zpd_objc::msg_send!(windows, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => zpd_objc::Id)
        }
    }

    /// Returns the accessibility identifier of the view hit at `y` points from the window bottom.
    fn hit(y: f64) -> String {
        Self::hit_at(200.0, y)
    }

    fn hit_at(x: f64, y: f64) -> String {
        // SAFETY: The window and its frame view are live; hitTest: takes the frame view's
        // superview coordinates, which are window coordinates for the root view.
        unsafe {
            let window = Self::window();
            let content = zpd_objc::msg_send!(window, zpd_objc::sel!("contentView"), () => zpd_objc::Id);
            let frame_view = zpd_objc::msg_send!(content, zpd_objc::sel!("superview"), () => zpd_objc::Id);
            let point = native::Point { x, y };
            let hit = zpd_objc::msg_send!(frame_view, zpd_objc::sel!("hitTest:"), ((point): native::Point) => zpd_objc::Id);
            if hit.is_null() {
                return String::new();
            }
            native::rust_string(zpd_objc::msg_send!(hit, zpd_objc::sel!("accessibilityIdentifier"), () => zpd_objc::Id))
        }
    }
}

fn main() {
    let application = Application::new().unwrap();
    let window = application.create_window().unwrap();
    let inset = Rc::new(Cell::new(None));
    let reported = inset.clone();
    window
        .set_content_layout_handler(move |_, title_bar_inset| reported.set(Some(title_bar_inset)))
        .unwrap();
    window.set_bounds(Rect::new(100.0, 100.0, 400.0, 300.0)).unwrap();
    window.set_title_bar(TitleBar::Hidden).unwrap();
    let content = window.content_view().unwrap();
    let canvas = CanvasView::new(&application).unwrap();
    content.add_subview(&canvas).unwrap();
    canvas.as_view().set_frame(content.bounds().unwrap()).unwrap();
    canvas.as_view().set_identifier(Some("canvas")).unwrap();
    let height = content.bounds().unwrap().height;

    // Verifies a hidden title bar leaves its area to the window for unmarked views.
    assert_ne!(TitleBarChecks::hit(height - 8.0), "canvas");
    // Verifies the content area below the title bar still reaches the view.
    assert_eq!(TitleBarChecks::hit(height / 2.0), "canvas");

    // Verifies a canvas delegate claims only the title bar points it reports.
    canvas.set_delegate(Rc::new(Tabs));
    assert_eq!(TitleBarChecks::hit_at(50.0, height - 8.0), "canvas");
    assert_ne!(TitleBarChecks::hit_at(200.0, height - 8.0), "canvas");

    // Verifies the content layout reports the height the title bar covers.
    let content = window.content_view().unwrap();
    content.set_needs_layout(true).unwrap();
    content.layout_subtree_if_needed().unwrap();
    assert!(inset.get().is_some_and(|inset| inset > 0.0));

    // Verifies a marked view receives clicks inside the title bar area.
    canvas.as_view().set_title_bar_interactive(true).unwrap();
    assert_eq!(TitleBarChecks::hit(height - 8.0), "canvas");

    // Verifies the standard title bar keeps content out of the title bar area.
    canvas.as_view().set_title_bar_interactive(false).unwrap();
    window.set_title_bar(TitleBar::Standard).unwrap();
    let content = window.content_view().unwrap();
    let content_height = content.bounds().unwrap().height;
    assert_eq!(TitleBarChecks::hit(content_height - 8.0), "canvas");

    // Verifies a standard title bar covers none of the content.
    content.set_needs_layout(true).unwrap();
    content.layout_subtree_if_needed().unwrap();
    assert_eq!(inset.get(), Some(0.0));
}
