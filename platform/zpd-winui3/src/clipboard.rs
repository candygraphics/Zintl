//! Plain text on the system clipboard.

use std::ffi::c_void;

use crate::{Result, check, ffi, string_from_ref};

/// The system clipboard, which Copy and Paste use.
///
/// Call it on the UI thread.
pub struct Clipboard;

impl Clipboard {
    /// Replaces the clipboard's contents with `text`.
    pub fn set_text(text: &str) -> Result<()> {
        // SAFETY: The string reference is valid for the call, which copies it.
        check(unsafe { ffi::zpd_winui3_clipboard_set_text(ffi::StringRef::new(text)) })
    }

    /// Returns the clipboard's plain text, or `None` when it holds none.
    pub fn text() -> Result<Option<String>> {
        unsafe extern "C" fn receive(data: *const c_void, value: ffi::StringRef) {
            // SAFETY: `data` is the Option below and `value` is valid for this call.
            unsafe { *data.cast_mut().cast::<Option<String>>() = Some(string_from_ref(value)) };
        }
        let mut text = None::<String>;
        // SAFETY: `receive` writes into `text` synchronously during this call.
        check(unsafe {
            ffi::zpd_winui3_clipboard_get_text(std::ptr::from_mut(&mut text).cast(), receive)
        })?;
        Ok(text)
    }
}
