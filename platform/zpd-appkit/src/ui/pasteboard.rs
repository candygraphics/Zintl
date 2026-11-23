//! Plain text on the general pasteboard.

use zpd_objc::Id;

use crate::native;

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    static NSPasteboardTypeString: Id;
}

/// `NSPasteboard.generalPasteboard`, the clipboard Copy and Paste use.
///
/// Call it on the main thread.
pub struct Pasteboard;

impl Pasteboard {
    /// Replaces the pasteboard's contents with `text`.
    pub fn set_string(text: &str) {
        let text = native::nsstring(text);
        // SAFETY: The general pasteboard is a live shared object; it copies the string, and
        // NSPasteboardTypeString is a constant NSString.
        unsafe {
            let pasteboard = Self::general();
            zpd_objc::msg_send!(pasteboard, zpd_objc::sel!("clearContents"), () => isize);
            zpd_objc::msg_send!(pasteboard, zpd_objc::sel!("setString:forType:"), ((text.as_ptr()): Id, (NSPasteboardTypeString): Id) => bool);
        }
    }

    /// Returns the pasteboard's plain text, or `None` when it holds none.
    pub fn string() -> Option<String> {
        // SAFETY: The general pasteboard is live; `stringForType:` returns nil or an
        // autoreleased NSString that is copied before this returns.
        unsafe {
            let text = zpd_objc::msg_send!(Self::general(), zpd_objc::sel!("stringForType:"), ((NSPasteboardTypeString): Id) => Id);
            (!text.is_null()).then(|| native::rust_string(text))
        }
    }

    /// Returns the shared general pasteboard.
    ///
    /// # Safety
    /// Call it on the main thread; the pasteboard is not retained.
    unsafe fn general() -> Id {
        // SAFETY: Guaranteed by the caller; the class method takes no arguments.
        unsafe {
            zpd_objc::msg_send!(zpd_objc::class!("NSPasteboard"), zpd_objc::sel!("generalPasteboard"), () => Id)
        }
    }
}
