// Checks how windows are sized by their content area and centered.
#[path = "../../src/native.rs"]
mod native;

use zpd_appkit::geometry::Rect;
use zpd_appkit::runloop::Application;
use zpd_appkit::ui::TitleBar;

struct WindowFrames;

impl WindowFrames {
    /// Returns the only window's frame, with AppKit's bottom-left origin.
    fn frame() -> native::Rect {
        // SAFETY: This isolated main-thread test creates one window.
        unsafe { zpd_objc::msg_send!(Self::window(), zpd_objc::sel!("frame"), () => native::Rect) }
    }

    /// Returns the visible frame of the screen the window is on.
    fn visible_screen() -> native::Rect {
        // SAFETY: The window is on screen, so it has a screen with a visible frame.
        unsafe {
            let screen = zpd_objc::msg_send!(Self::window(), zpd_objc::sel!("screen"), () => zpd_objc::Id);
            zpd_objc::msg_send!(screen, zpd_objc::sel!("visibleFrame"), () => native::Rect)
        }
    }

    fn window() -> zpd_objc::Id {
        // SAFETY: Reads the shared application's first window on the main thread.
        unsafe {
            let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => zpd_objc::Id);
            let windows = zpd_objc::msg_send!(app, zpd_objc::sel!("windows"), () => zpd_objc::Id);
            zpd_objc::msg_send!(windows, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => zpd_objc::Id)
        }
    }

    fn top(frame: native::Rect) -> f64 {
        frame.origin.y + frame.size.height
    }
}

fn main() {
    let application = Application::new().unwrap();
    let window = application.create_window().unwrap();
    window.set_bounds(Rect::new(100.0, 100.0, 400.0, 300.0)).unwrap();
    let before = WindowFrames::frame();

    // Verifies the content area takes the requested size and the frame keeps its top-left corner.
    window.set_content_size(320.0, 200.0).unwrap();
    let content = window.content_view().unwrap().bounds().unwrap();
    let after = WindowFrames::frame();
    assert_eq!((content.width, content.height), (320.0, 200.0));
    assert_eq!(after.origin.x, before.origin.x);
    assert_eq!(WindowFrames::top(after), WindowFrames::top(before));
    // Verifies a standard title bar makes the frame taller than the content.
    assert!(after.size.height > 200.0);

    // Verifies content under a hidden title bar fills the frame at the requested size.
    window.set_title_bar(TitleBar::Hidden).unwrap();
    window.set_content_size(320.0, 200.0).unwrap();
    let content = window.content_view().unwrap().bounds().unwrap();
    assert_eq!((content.width, content.height), (320.0, 200.0));
    assert_eq!(WindowFrames::frame().size.height, 200.0);

    // Verifies centering places the frame in the middle of the screen horizontally.
    window.center().unwrap();
    let frame = WindowFrames::frame();
    let screen = WindowFrames::visible_screen();
    let middle = frame.origin.x + frame.size.width / 2.0;
    let screen_middle = screen.origin.x + screen.size.width / 2.0;
    assert!((middle - screen_middle).abs() <= 1.0);
}
