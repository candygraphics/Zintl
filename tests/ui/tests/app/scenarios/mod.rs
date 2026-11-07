//! UI test scenarios, each a root view the XCUITests launch by name.

use zintl_ui_desktop::{App, AppError};

use crate::launch::LaunchOptions;

/// Declares the scenario modules and maps each launch name to a function building its root view.
macro_rules! scenarios {
    ($($module:ident => $name:literal: $make:expr,)*) => {
        $(pub mod $module;)*

        /// The scenarios the app can launch.
        pub struct Scenarios;

        impl Scenarios {
            /// Runs the scenario called `name`, or returns `None` when there is none.
            pub fn run(name: &str, options: &LaunchOptions) -> Option<Result<(), AppError>> {
                match name {
                    $($name => Some(App::new(($make)(options)).run()),)*
                    _ => None,
                }
            }
        }
    };
}

scenarios! {
    arrangements => "arrangements": |_| arrangements::MainView::new(),
    canvas_action => "canvas-action": |_| canvas_action::MainView::new(),
    controls => "controls": |_| controls::MainView::new(),
    document => "document": |_| document::MainView::new(),
    full_size_sidebar => "full-size-sidebar": |_| full_size_sidebar::MainView::default(),
    full_width_stack => "full-width-stack": |_| full_width_stack::MainView,
    hstack => "hstack": |_| hstack::MainView,
    overflow => "overflow": |_| overflow::MainView,
    perform_main => "perform-main": |_| perform_main::MainView::new(),
    scroll_status => "scroll-status": |_| scroll_status::MainView::default(),
    store_sender => "store-sender": |_| store_sender::MainView::new(),
    table => "table": |_| table::MainView::new(),
    text_commit => "text-commit": |_| text_commit::MainView::new(),
    text_field => "text-field": |_| text_field::MainView::new(),
    truncation => "truncation": |_| truncation::MainView,
    wgpu_first_frame => "wgpu-first-frame": |_| wgpu_first_frame::MainView::new(),
    window_close => "window-close": |_| window_close::MainView,
}
