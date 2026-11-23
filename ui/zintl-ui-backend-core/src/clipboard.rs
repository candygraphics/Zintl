//! Plain text on the system clipboard.

use std::cell::RefCell;

/// The platform clipboard a backend gives to [`crate::Context::clipboard`].
pub trait ClipboardAccess {
    /// Replaces the clipboard's contents with `text`.
    fn set_text(&self, text: &str);

    /// Returns the clipboard's plain text, or `None` when it holds none.
    fn text(&self) -> Option<String>;
}

/// The system clipboard, for actions such as "Copy" buttons.
///
/// ```ignore
/// Button::new("Copy").on_click(move |cx| {
///     let text = cx.get(result).to_string();
///     cx.clipboard().set_text(text);
/// })
/// ```
#[derive(Clone, Copy)]
pub struct Clipboard<'a> {
    access: &'a dyn ClipboardAccess,
}

impl<'a> Clipboard<'a> {
    pub(crate) fn new(access: &'a dyn ClipboardAccess) -> Self {
        Self { access }
    }

    /// Replaces the clipboard's contents with `text`.
    ///
    /// The write may be dropped while another app holds the clipboard open.
    pub fn set_text(&self, text: impl AsRef<str>) {
        self.access.set_text(text.as_ref());
    }

    /// Returns the clipboard's plain text, or `None` when it holds none, such as an image.
    pub fn text(&self) -> Option<String> {
        self.access.text()
    }
}

/// A clipboard kept in memory, so tests do not touch the user's clipboard.
#[derive(Debug, Default)]
pub struct MemoryClipboard {
    text: RefCell<Option<String>>,
}

impl ClipboardAccess for MemoryClipboard {
    fn set_text(&self, text: &str) {
        *self.text.borrow_mut() = Some(text.to_owned());
    }

    fn text(&self) -> Option<String> {
        self.text.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_clipboard_keeps_the_last_text() {
        // Verifies the in-memory clipboard starts empty and returns the last text written.
        let memory = MemoryClipboard::default();
        let clipboard = Clipboard::new(&memory);
        assert_eq!(clipboard.text(), None);

        clipboard.set_text("first");
        clipboard.set_text(String::from("second"));
        assert_eq!(clipboard.text().as_deref(), Some("second"));
    }
}
