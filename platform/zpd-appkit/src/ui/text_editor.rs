use std::sync::OnceLock;

use crate::actor::WindowEventKind;
use crate::graphics::Font;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, Strong};

use super::UiError;
use super::callback;
use super::rich_text::{RichParagraph, RichRun, RichStyle, RichText};
use super::view::{AsView, ViewActor, ViewRef};

/// Applies the paragraph style in the sender's tag to the selected paragraphs.
///
/// The change goes through `shouldChangeTextInRange:replacementString:`, so
/// it is undoable and reported like typing.
unsafe extern "C" fn apply_paragraph_style(object: Id, _: zpd_objc::Sel, sender: Id) {
    zpd_objc::abort_on_unwind(|| {
        // SAFETY: AppKit sends this action to the live text view with a menu item or nil sender.
        unsafe {
            let tag = if sender.is_null() {
                0
            } else {
                zpd_objc::msg_send!(sender, zpd_objc::sel!("tag"), () => native::Integer)
            };
            let Some(target) = RichParagraph::from_tag(tag) else {
                return;
            };
            // A list style on a paragraph that already has it turns the list off.
            let current = TextEditor::paragraph_at_selection(object);
            let paragraph = if target.is_list() && current.tag() == target.tag() {
                RichParagraph::Body
            } else {
                target
            };
            let selection =
                zpd_objc::msg_send!(object, zpd_objc::sel!("selectedRange"), () => native::Range);
            TextEditor::restyle(object, selection, |_| paragraph, true);
        }
    });
}

/// Enables paragraph style items in rich editors and checks the current style.
unsafe extern "C" fn validate_menu_item(object: Id, selector: zpd_objc::Sel, item: Id) -> bool {
    zpd_objc::abort_on_unwind(|| {
        // SAFETY: AppKit validates live menu items against the live text view.
        unsafe {
            let action = zpd_objc::msg_send!(item, zpd_objc::sel!("action"), () => zpd_objc::Sel);
            if action != zpd_objc::sel!("zintlApplyParagraphStyle:") {
                return zpd_objc::msg_send_super!(object, zpd_objc::class!("NSTextView"), selector, ((item): Id) => bool);
            }
            let tag = zpd_objc::msg_send!(item, zpd_objc::sel!("tag"), () => native::Integer);
            let current = TextEditor::paragraph_at_selection(object);
            let state = if current.tag() == tag {
                native::NS_CONTROL_STATE_VALUE_ON
            } else {
                0
            };
            zpd_objc::msg_send!(item, zpd_objc::sel!("setState:"), ((state): native::Integer) => ());
            zpd_objc::msg_send!(object, zpd_objc::sel!("isRichText"), () => bool)
                && zpd_objc::msg_send!(object, zpd_objc::sel!("isEditable"), () => bool)
        }
    })
}

/// A scrolling, multiline `NSTextView` with undo and a find bar.
///
/// In rich mode it reads and writes [`RichRun`]s and responds to
/// `zintlApplyParagraphStyle:`, whose sender's tag is a [`RichParagraph`].
pub struct TextEditor {
    view: ViewActor,
}

impl TextEditor {
    fn text_view_class() -> zpd_objc::Class {
        static CLASS: OnceLock<usize> = OnceLock::new();
        *CLASS.get_or_init(|| {
            zpd_objc::decl!(ZpdTextView: [zpd_objc::class!("NSTextView")] {
                methods {
                    "zintlApplyParagraphStyle:": "v@:@" => apply_paragraph_style,
                    "validateMenuItem:": "B@:@" => validate_menu_item,
                    "drawViewBackgroundInRect:": "v@:{CGRect={CGPoint=dd}{CGSize=dd}}" => super::text_lists::draw_background,
                    "mouseDown:": "v@:@" => super::text_lists::mouse_down,
                    "insertNewline:": "v@:@" => super::text_lists::insert_newline,
                }
            }) as usize
        }) as zpd_objc::Class
    }

    pub fn new(application: &Application) -> Result<Self, UiError> {
        // SAFETY: The scroll view and text view are created and configured on the main thread.
        let (scroll, text) = unsafe {
            let scroll = native::alloc_init(zpd_objc::class!("NSScrollView"));
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setHasVerticalScroller:"), ((true): bool) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setAutohidesScrollers:"), ((true): bool) => ());
            let allocated =
                zpd_objc::msg_send!(Self::text_view_class(), zpd_objc::sel!("alloc"), () => Id);
            let frame = native::Rect::default();
            let text = Strong::from_retained(zpd_objc::msg_send!(allocated, zpd_objc::sel!("initWithFrame:"), ((frame): native::Rect) => Id))
                .ok_or(UiError::NativeCreationFailed)?;
            let text_view = text.as_ptr();
            let unbounded = native::Size {
                width: f64::from(f32::MAX),
                height: f64::from(f32::MAX),
            };
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setMinSize:"), ((native::Size::default()): native::Size) => ());
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setMaxSize:"), ((unbounded): native::Size) => ());
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setVerticallyResizable:"), ((true): bool) => ());
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setHorizontallyResizable:"), ((false): bool) => ());
            // NSViewWidthSizable keeps the text view as wide as the clip view.
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setAutoresizingMask:"), ((2_u64): u64) => ());
            let container =
                zpd_objc::msg_send!(text_view, zpd_objc::sel!("textContainer"), () => Id);
            zpd_objc::msg_send!(container, zpd_objc::sel!("setWidthTracksTextView:"), ((true): bool) => ());
            // List markers are positioned with TextKit 1 line fragments.
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("layoutManager"), () => Id);
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setRichText:"), ((false): bool) => ());
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setAllowsUndo:"), ((true): bool) => ());
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setUsesFindBar:"), ((true): bool) => ());
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setIncrementalSearchingEnabled:"), ((true): bool) => ());
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
            zpd_objc::msg_send!(text_view, zpd_objc::sel!("setTextContainerInset:"), ((native::Size { width: 12.0, height: 12.0 }): native::Size) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDocumentView:"), ((text_view): Id) => ());
            (scroll, text)
        };
        let view = ViewActor::new(application, scroll);
        let emit = callback::emitter(view.actor());
        callback::install_observer(
            view.actor(),
            "NSTextDidChangeNotification",
            text.as_ptr(),
            move |notification| {
                // SAFETY: Text notifications carry the live NSTextView as their object.
                let kind = unsafe {
                    let text =
                        zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id);
                    if zpd_objc::msg_send!(text, zpd_objc::sel!("isRichText"), () => bool) {
                        // List numbers below the edit may change, so markers redraw everywhere.
                        zpd_objc::msg_send!(text, zpd_objc::sel!("setNeedsDisplay:"), ((true): bool) => ());
                        let storage =
                            zpd_objc::msg_send!(text, zpd_objc::sel!("textStorage"), () => Id);
                        WindowEventKind::RichTextChanged {
                            runs: RichText::runs(storage),
                        }
                    } else {
                        WindowEventKind::TextChanged {
                            value: native::rust_string(
                                zpd_objc::msg_send!(text, zpd_objc::sel!("string"), () => Id),
                            ),
                        }
                    }
                };
                emit(kind);
            },
        )?;
        let editor = Self { view };
        editor.set_font(None)?;
        Ok(editor)
    }

    /// Returns the paragraph style at the start of the selection.
    ///
    /// # Safety
    /// `text` must be a live text view created by [`TextEditor::new`].
    pub(crate) unsafe fn paragraph_at_selection(text: Id) -> RichParagraph {
        // SAFETY: Guaranteed by the caller; indexes are checked against the length.
        unsafe {
            let storage = zpd_objc::msg_send!(text, zpd_objc::sel!("textStorage"), () => Id);
            let length = zpd_objc::msg_send!(storage, zpd_objc::sel!("length"), () => u64);
            let selection =
                zpd_objc::msg_send!(text, zpd_objc::sel!("selectedRange"), () => native::Range);
            let attributes = if selection.location < length {
                let mut range = native::Range::default();
                zpd_objc::msg_send!(storage, zpd_objc::sel!("attributesAtIndex:effectiveRange:"), ((selection.location): u64, (&mut range): *mut native::Range) => Id)
            } else {
                zpd_objc::msg_send!(text, zpd_objc::sel!("typingAttributes"), () => Id)
            };
            RichText::style(attributes).paragraph
        }
    }

    /// Maps the paragraph style of every paragraph that `range` touches.
    ///
    /// The change goes through `shouldChangeTextInRange:replacementString:`, so
    /// it is undoable and reported like typing. With `typing`, text typed next
    /// uses the new style, which also styles an empty last line.
    ///
    /// # Safety
    /// `text` must be a live rich text view created by [`TextEditor::new`].
    pub(crate) unsafe fn restyle(
        text: Id,
        range: native::Range,
        map: impl Fn(RichParagraph) -> RichParagraph,
        typing: bool,
    ) {
        // SAFETY: Guaranteed by the caller; ranges come from the text view's own string.
        unsafe {
            let storage = zpd_objc::msg_send!(text, zpd_objc::sel!("textStorage"), () => Id);
            let string = zpd_objc::msg_send!(text, zpd_objc::sel!("string"), () => Id);
            let selection =
                zpd_objc::msg_send!(text, zpd_objc::sel!("selectedRange"), () => native::Range);
            let paragraphs = zpd_objc::msg_send!(string, zpd_objc::sel!("paragraphRangeForRange:"), ((range): native::Range) => native::Range);
            let mut last = map(Self::paragraph_at_selection(text));
            if paragraphs.length > 0 {
                let existing = zpd_objc::msg_send!(storage, zpd_objc::sel!("attributedSubstringFromRange:"), ((paragraphs): native::Range) => Id);
                let runs = RichText::runs(existing)
                    .into_iter()
                    .map(|run| RichRun {
                        style: RichStyle {
                            paragraph: map(run.style.paragraph),
                            ..run.style
                        },
                        ..run
                    })
                    .collect::<Vec<_>>();
                if let Some(run) = runs.last() {
                    last = run.style.paragraph;
                }
                let restyled = RichText::attributed_string(&runs);
                if zpd_objc::msg_send!(text, zpd_objc::sel!("shouldChangeTextInRange:replacementString:"), ((paragraphs): native::Range, (zpd_objc::NIL): Id) => bool)
                {
                    zpd_objc::msg_send!(storage, zpd_objc::sel!("replaceCharactersInRange:withAttributedString:"), ((paragraphs): native::Range, (restyled.as_ptr()): Id) => ());
                    zpd_objc::msg_send!(text, zpd_objc::sel!("didChangeText"), () => ());
                    zpd_objc::msg_send!(text, zpd_objc::sel!("setSelectedRange:"), ((selection): native::Range) => ());
                }
            }
            if typing {
                let attributes = RichText::attributes(&RichStyle {
                    paragraph: last,
                    ..RichStyle::default()
                });
                zpd_objc::msg_send!(text, zpd_objc::sel!("setTypingAttributes:"), ((attributes.as_ptr()): Id) => ());
            }
        }
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

    /// Clears undo history after the content was replaced from code.
    ///
    /// # Safety
    /// `text` must be a live text view.
    unsafe fn forget_undo(text: Id) {
        // SAFETY: Guaranteed by the caller; the undo manager may be nil.
        unsafe {
            let undo = zpd_objc::msg_send!(text, zpd_objc::sel!("undoManager"), () => Id);
            if !undo.is_null() {
                zpd_objc::msg_send!(undo, zpd_objc::sel!("removeAllActions"), () => ());
            }
        }
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
            // SAFETY: The live NSTextView accepts an NSString.
            unsafe {
                zpd_objc::msg_send!(text, zpd_objc::sel!("setString:"), ((value.as_ptr()): Id) => ());
                Self::forget_undo(text);
            }
        })
    }

    /// Switches between plain text and styled runs.
    pub fn set_rich(&self, rich: bool) -> Result<(), UiError> {
        self.with_text(|text| {
            // SAFETY: The live NSTextView accepts the flag and typing attributes.
            unsafe {
                zpd_objc::msg_send!(text, zpd_objc::sel!("setRichText:"), ((rich): bool) => ());
                if rich {
                    let typing = RichText::attributes(&RichStyle::default());
                    zpd_objc::msg_send!(text, zpd_objc::sel!("setTypingAttributes:"), ((typing.as_ptr()): Id) => ());
                }
            }
        })
    }

    pub fn rich_text(&self) -> Result<Vec<RichRun>, UiError> {
        self.with_text(|text| {
            // SAFETY: The live NSTextView owns its text storage.
            unsafe {
                RichText::runs(zpd_objc::msg_send!(text, zpd_objc::sel!("textStorage"), () => Id))
            }
        })
    }

    /// Replaces the content with `runs` without reporting a change.
    ///
    /// Like [`TextEditor::set_string_value`], the insertion point moves to the
    /// end and undo history is cleared.
    pub fn set_rich_text(&self, runs: &[RichRun]) -> Result<(), UiError> {
        let attributed = RichText::attributed_string(runs);
        let typing = RichText::attributes(
            &runs
                .last()
                .map_or_else(RichStyle::default, |run| run.style.clone()),
        );
        self.with_text(|text| {
            // SAFETY: The live NSTextView owns its text storage; the strings are live.
            unsafe {
                let storage = zpd_objc::msg_send!(text, zpd_objc::sel!("textStorage"), () => Id);
                zpd_objc::msg_send!(storage, zpd_objc::sel!("setAttributedString:"), ((attributed.as_ptr()): Id) => ());
                let end = zpd_objc::msg_send!(storage, zpd_objc::sel!("length"), () => u64);
                zpd_objc::msg_send!(text, zpd_objc::sel!("setSelectedRange:"), ((native::Range { location: end, length: 0 }): native::Range) => ());
                zpd_objc::msg_send!(text, zpd_objc::sel!("setTypingAttributes:"), ((typing.as_ptr()): Id) => ());
                Self::forget_undo(text);
            }
        })
    }

    /// Sets the plain-text font; `None` uses the 14-point system font.
    pub fn set_font(&self, font: Option<&Font>) -> Result<(), UiError> {
        self.with_text(|text| {
            // SAFETY: The live NSTextView accepts an NSFont that `font` keeps alive.
            unsafe {
                let font = match font {
                    Some(font) => font.as_ptr(),
                    None => {
                        zpd_objc::msg_send!(zpd_objc::class!("NSFont"), zpd_objc::sel!("systemFontOfSize:"), ((14.0): f64) => Id)
                    }
                };
                zpd_objc::msg_send!(text, zpd_objc::sel!("setFont:"), ((font): Id) => ());
            }
        })
    }

    /// Makes the text view its window's first responder.
    pub fn focus(&self) -> Result<(), UiError> {
        self.with_text(|text| {
            // SAFETY: The live text view and its window, which may be nil, accept the message.
            unsafe {
                let window = zpd_objc::msg_send!(text, zpd_objc::sel!("window"), () => Id);
                if !window.is_null() {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("makeFirstResponder:"), ((text): Id) => bool);
                }
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
