use crate::actor::WindowEventKind;
use crate::graphics::Font;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, Strong};

use super::UiError;
use super::callback;
use super::view::{AsView, ViewActor, ViewRef};

/// A scrolling, multiline plain-text `NSTextView` with undo.
pub struct TextEditor {
    view: ViewActor,
}

impl TextEditor {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        // SAFETY: scrollableTextView returns an autoreleased NSScrollView whose
        // document view is an NSTextView; both are configured on the main thread.
        let scroll = unsafe {
            let scroll = Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("NSTextView"), zpd_objc::sel!("scrollableTextView"), () => Id))
                .ok_or(UiError::NativeCreationFailed)?;
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setAutohidesScrollers:"), ((true): bool) => ());
            let text =
                zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("documentView"), () => Id);
            zpd_objc::msg_send!(text, zpd_objc::sel!("setRichText:"), ((false): bool) => ());
            zpd_objc::msg_send!(text, zpd_objc::sel!("setAllowsUndo:"), ((true): bool) => ());
            zpd_objc::msg_send!(text, zpd_objc::sel!("setUsesFindBar:"), ((true): bool) => ());
            zpd_objc::msg_send!(text, zpd_objc::sel!("setIncrementalSearchingEnabled:"), ((true): bool) => ());
            zpd_objc::msg_send!(text, zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
            zpd_objc::msg_send!(text, zpd_objc::sel!("setTextContainerInset:"), ((native::Size { width: 12.0, height: 12.0 }): native::Size) => ());
            scroll
        };
        let text = Self::text_view_of(scroll.as_ptr());
        let view = ViewActor::new(application, scroll);
        let emit = callback::emitter(view.actor());
        callback::install_observer(
            view.actor(),
            "NSTextDidChangeNotification",
            text,
            move |notification| {
                // SAFETY: Text notifications carry the live NSTextView as their object.
                let value = unsafe {
                    let text =
                        zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id);
                    native::rust_string(
                        zpd_objc::msg_send!(text, zpd_objc::sel!("string"), () => Id),
                    )
                };
                emit(WindowEventKind::TextChanged { value });
            },
        )?;
        let editor = Self { view };
        editor.set_font(None)?;
        Ok(editor)
    }

    /// Returns the text view inside `scroll`, which the scroll view retains.
    fn text_view_of(scroll: Id) -> Id {
        // SAFETY: `scroll` is the live NSScrollView created by `new`.
        unsafe { zpd_objc::msg_send!(scroll, zpd_objc::sel!("documentView"), () => Id) }
    }

    fn with_text<R>(&self, operation: impl FnOnce(Id) -> R) -> Result<R, UiError> {
        self.view
            .as_view()
            .with(|scroll| operation(Self::text_view_of(scroll)))
    }

    pub fn string_value(&self) -> Result<String, UiError> {
        self.with_text(|text| {
            // SAFETY: The live NSTextView returns its backing string.
            unsafe {
                native::rust_string(zpd_objc::msg_send!(text, zpd_objc::sel!("string"), () => Id))
            }
        })
    }

    /// Replaces the text without reporting a change; the insertion point moves to the end.
    ///
    /// Undo history is cleared, since it describes the replaced text.
    pub fn set_string_value(&self, value: &str) -> Result<(), UiError> {
        let value = native::nsstring(value);
        self.with_text(|text| {
            // SAFETY: The live NSTextView accepts an NSString; its undo manager may be nil.
            unsafe {
                zpd_objc::msg_send!(text, zpd_objc::sel!("setString:"), ((value.as_ptr()): Id) => ());
                let undo = zpd_objc::msg_send!(text, zpd_objc::sel!("undoManager"), () => Id);
                if !undo.is_null() {
                    zpd_objc::msg_send!(undo, zpd_objc::sel!("removeAllActions"), () => ());
                }
            }
        })
    }

    /// Sets the font; `None` uses the 14-point system font.
    pub fn set_font(&self, font: Option<&Font>) -> Result<(), UiError> {
        self.with_text(|text| {
            // SAFETY: The live NSTextView accepts an NSFont that `font` keeps alive.
            unsafe {
                let font = match font {
                    Some(font) => font.as_ptr(),
                    None => zpd_objc::msg_send!(zpd_objc::class!("NSFont"), zpd_objc::sel!("systemFontOfSize:"), ((14.0): f64) => Id),
                };
                zpd_objc::msg_send!(text, zpd_objc::sel!("setFont:"), ((font): Id) => ());
            }
        })
    }

    pub fn set_editable(&self, editable: bool) -> Result<(), UiError> {
        self.with_text(|text| {
            // SAFETY: The live NSTextView accepts the flag.
            unsafe {
                zpd_objc::msg_send!(text, zpd_objc::sel!("setEditable:"), ((editable): bool) => ());
            }
        })
    }
}

impl AsView for TextEditor {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}
