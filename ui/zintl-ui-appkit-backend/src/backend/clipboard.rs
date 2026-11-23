use zintl_ui_backend_core::ClipboardAccess;
use zpd_appkit::ui::Pasteboard;

/// The general pasteboard, which Copy and Paste use.
pub(super) struct SystemClipboard;

impl ClipboardAccess for SystemClipboard {
    fn set_text(&self, text: &str) {
        Pasteboard::set_string(text);
    }

    fn text(&self) -> Option<String> {
        Pasteboard::string()
    }
}
