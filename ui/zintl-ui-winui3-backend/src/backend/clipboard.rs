use zintl_ui_backend_core::ClipboardAccess;
use zpd_winui3::Clipboard;

/// The system clipboard, which Copy and Paste use.
///
/// Writes are dropped and reads return `None` while another app holds it open.
pub(super) struct SystemClipboard;

impl ClipboardAccess for SystemClipboard {
    fn set_text(&self, text: &str) {
        let _ = Clipboard::set_text(text);
    }

    fn text(&self) -> Option<String> {
        Clipboard::text().ok().flatten()
    }
}
