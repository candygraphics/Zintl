//! The app runner and its platform backend.

use zintl_ui::composer::Composer;
use zintl_ui::element::IntoElement;
use zintl_ui_native::RenderNode;

#[cfg(target_os = "macos")]
pub use zintl_ui_appkit_backend::AppError;
#[cfg(target_os = "windows")]
pub use zintl_ui_winui3_backend::AppError;

#[cfg(target_os = "macos")]
type DesktopBackend = zintl_ui_appkit_backend::AppKitBackend;

#[cfg(target_os = "windows")]
type DesktopBackend = zintl_ui_winui3_backend::WinUI3Backend;

// Other platforms have no native backend yet, so apps build but cannot run.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
type DesktopBackend = zintl_ui_backend_core::HeadlessBackend;

/// Mounts views on the platform's native backend and runs them.
///
/// Tests inspect views with [`crate::testing::Harness`] instead.
pub struct App {
    composer: Composer<RenderNode, DesktopBackend>,
    on_exit: Option<Box<dyn FnOnce()>>,
}

impl App {
    pub fn new<E>(root: E) -> Self
    where
        E: IntoElement<Output = RenderNode>,
    {
        #[allow(unused_mut)]
        let mut backend = DesktopBackend::new();
        // Native backends install the standard menus until the app declares its own.
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        backend.set_default_main_menu(zintl_ui_native::MainMenu::standard().commands());
        let mut composer = Composer::new(backend);
        composer.mount(root);
        Self {
            composer,
            on_exit: None,
        }
    }

    /// Runs once before the process ends, whether the last window closed or
    /// the user quit, for example to finish saving.
    ///
    /// On macOS, ⌘Q exits the process without returning from [`App::run`],
    /// so work after `run` is not enough.
    pub fn on_exit(mut self, action: impl FnOnce() + 'static) -> Self {
        self.on_exit = Some(Box::new(action));
        self
    }

    #[cfg(target_os = "macos")]
    pub fn run(self) -> Result<(), AppError> {
        let on_exit = self.on_exit.unwrap_or_else(|| Box::new(|| {}));
        zintl_ui_appkit_backend::run_composer(self.composer, on_exit)
    }

    #[cfg(target_os = "windows")]
    pub fn run(self) -> Result<(), AppError> {
        let on_exit = self.on_exit.unwrap_or_else(|| Box::new(|| {}));
        zintl_ui_winui3_backend::run_composer(self.composer, on_exit)
    }
}
