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
                    $($name => Some(App::new(Self::build(options, $make)).run()),)*
                    _ => None,
                }
            }

            /// Calls `make`, which lets each scenario's closure infer its parameter type.
            fn build<V>(options: &LaunchOptions, make: impl FnOnce(&LaunchOptions) -> V) -> V {
                make(options)
            }
        }
    };
}

scenarios! {
    alert => "alert": |_| alert::MainView::new(),
    arrangements => "arrangements": |_| arrangements::MainView::new(),
    buttons => "buttons": |_| buttons::MainView::new(),
    canvas_action => "canvas-action": |_| canvas_action::MainView::new(),
    canvas_input => "canvas-input": |_| canvas_input::MainView::new(),
    color_panel => "color-panel": |_| color_panel::MainView::new(),
    context_menu => "context-menu": |_| context_menu::MainView::new(),
    controls => "controls": |_| controls::MainView::new(),
    document => "document": |_| document::MainView::new(),
    file_dialog => "file-dialog": |options| {
        file_dialog::MainView::new(options.variable("ZINTL_UI_TEST_DIRECTORY"))
    },
    find_menu => "find-menu": |_| find_menu::MainView::new(),
    full_size_sidebar => "full-size-sidebar": |_| full_size_sidebar::MainView::default(),
    full_width_stack => "full-width-stack": |_| full_width_stack::MainView,
    grid_variants => "grid-variants": |_| grid_variants::MainView,
    help => "help": |_| help::MainView,
    hstack => "hstack": |_| hstack::MainView,
    indicators => "indicators": |_| indicators::MainView::new(),
    list => "list": |_| list::MainView::new(),
    main_menu => "main-menu": |options| {
        main_menu::MainView::new(options.variable("ZINTL_UI_TEST_MENU_LANGUAGE").as_deref() == Some("ja"))
    },
    overflow => "overflow": |_| overflow::MainView,
    perform_main => "perform-main": |_| perform_main::MainView::new(),
    pickers => "pickers": |_| pickers::MainView::new(),
    popover => "popover": |_| popover::MainView::new(),
    scroll_status => "scroll-status": |_| scroll_status::MainView::default(),
    sidebar_features => "sidebar-features": |_| sidebar_features::MainView::new(),
    sliders => "sliders": |_| sliders::MainView::new(),
    split_view => "split-view": |_| split_view::MainView::new(),
    store_sender => "store-sender": |_| store_sender::MainView::new(),
    table => "table": |_| table::MainView::new(),
    tabs => "tabs": |_| tabs::MainView::new(),
    text_commit => "text-commit": |_| text_commit::MainView::new(),
    text_editor => "text-editor": |_| text_editor::MainView::new(),
    text_field => "text-field": |_| text_field::MainView::new(),
    text_inputs => "text-inputs": |_| text_inputs::MainView::new(),
    text_styles => "text-styles": |_| text_styles::MainView,
    toggles => "toggles": |_| toggles::MainView::new(),
    toolbar => "toolbar": |_| toolbar::MainView::new(),
    truncation => "truncation": |_| truncation::MainView,
    wgpu_first_frame => "wgpu-first-frame": |_| wgpu_first_frame::MainView::new(),
    wgpu_input => "wgpu-input": |_| wgpu_input::MainView::new(),
    window_close => "window-close": |_| window_close::MainView,
    windows => "windows": |_| windows::MainView::new(),
}
