use crate::actor::WindowEventKind;
use crate::graphics::Font;
use crate::native;
use crate::runloop::Application;
use zpd_objc::Strong;

use super::callback;
use super::view::{AsView, ViewActor, ViewRef};
use super::{Color, UiError};

/// A text color; all but `Custom` follow the system appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextColor {
    #[default]
    Label,
    SecondaryLabel,
    TertiaryLabel,
    Accent,
    Custom(Color),
}

impl TextColor {
    /// Returns an autoreleased `NSColor`.
    fn native(self) -> zpd_objc::Id {
        let name = match self {
            Self::Label => zpd_objc::sel!("labelColor"),
            Self::SecondaryLabel => zpd_objc::sel!("secondaryLabelColor"),
            Self::TertiaryLabel => zpd_objc::sel!("tertiaryLabelColor"),
            Self::Accent => zpd_objc::sel!("controlAccentColor"),
            Self::Custom(color) => return color.native(),
        };
        // SAFETY: The NSColor class methods take no arguments and return autoreleased colors.
        unsafe { zpd_objc::msg_send!(zpd_objc::class!("NSColor"), name, () => zpd_objc::Id) }
    }
}

/// Horizontal text alignment; `Leading` follows the writing direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlignment {
    #[default]
    Leading,
    Center,
    Trailing,
}

impl TextAlignment {
    /// Returns the `NSTextAlignment` value, whose numbering differs on Intel Macs.
    fn native(self) -> isize {
        const NATURAL: isize = 4;
        #[cfg(target_arch = "x86_64")]
        const CENTER_RIGHT: (isize, isize) = (2, 1);
        #[cfg(not(target_arch = "x86_64"))]
        const CENTER_RIGHT: (isize, isize) = (1, 2);
        match self {
            Self::Leading => NATURAL,
            Self::Center => CENTER_RIGHT.0,
            Self::Trailing => CENTER_RIGHT.1,
        }
    }
}

pub struct TextField {
    view: ViewActor,
}
impl TextField {
    pub fn with_string(application: &Application, value: &str) -> Result<Self, UiError> {
        Self::create(application, value, false)
    }
    pub fn label_with_string(application: &Application, value: &str) -> Result<Self, UiError> {
        Self::create(application, value, true)
    }
    fn create(application: &Application, value: &str, label: bool) -> Result<Self, UiError> {
        let value = native::nsstring(value);
        // SAFETY: NSTextField alloc/init produces a retained object.
        let native = unsafe {
            let object = zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSTextField"), zpd_objc::sel!("alloc"), () => zpd_objc::Id), zpd_objc::sel!("init"), () => zpd_objc::Id);
            let object = Strong::from_retained(object).ok_or(UiError::NativeCreationFailed)?;
            zpd_objc::msg_send!(object.as_ptr(), zpd_objc::sel!("setStringValue:"), ((value.as_ptr()): zpd_objc::Id) => ());
            if label {
                zpd_objc::msg_send!(object.as_ptr(), zpd_objc::sel!("setEditable:"), ((false): bool) => ());
                zpd_objc::msg_send!(object.as_ptr(), zpd_objc::sel!("setSelectable:"), ((false): bool) => ());
                zpd_objc::msg_send!(object.as_ptr(), zpd_objc::sel!("setBordered:"), ((false): bool) => ());
                zpd_objc::msg_send!(object.as_ptr(), zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
            }
            object
        };
        let view = ViewActor::new(application, native);
        application
            .tree()
            .add_teardown(view.actor(), |field| unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setDelegate:"), ((zpd_objc::NIL): zpd_objc::Id) => ())
            })
            .map_err(UiError::from)?;
        if !label {
            let actor = view.actor().clone();
            let target = callback::target(move |notification| unsafe {
                let field =
                    zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => zpd_objc::Id);
                let value = native::rust_string(
                    zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => zpd_objc::Id),
                );
                if let Some(tree) = actor.tree_handle() {
                    tree.emit(&actor, WindowEventKind::TextChanged { value });
                }
            });
            let tree = view.actor().tree_handle().ok_or(UiError::Closed)?;
            let target = tree
                .replace_owned(view.actor(), "delegate", target)
                .map_err(UiError::from)?;
            tree.add_teardown(&target, |target| unsafe { callback::release(target) })
                .map_err(UiError::from)?;
            view.as_view().with(|field| {
                target.with(|target| unsafe {
                    zpd_objc::msg_send!(field, zpd_objc::sel!("setDelegate:"), ((target): zpd_objc::Id) => ())
                })
            })??;
        }
        Ok(Self { view })
    }
    pub fn set_string_value(&self, value: &str) -> Result<(), UiError> {
        let value = native::nsstring(value);
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setStringValue:"), ((value.as_ptr()): zpd_objc::Id) => ())
        })
    }
    pub fn string_value(&self) -> Result<String, UiError> {
        self.view.as_view().with(|field| unsafe {
            native::rust_string(
                zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => zpd_objc::Id),
            )
        })
    }
    pub fn set_placeholder_string(&self, value: Option<&str>) -> Result<(), UiError> {
        let value = value.map(native::nsstring);
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setPlaceholderString:"), ((value.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): zpd_objc::Id) => ());
        })
    }
    pub fn set_editable(&self, editable: bool) -> Result<(), UiError> {
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setEditable:"), ((editable): bool) => ())
        })
    }
    pub fn set_selectable(&self, selectable: bool) -> Result<(), UiError> {
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setSelectable:"), ((selectable): bool) => ())
        })
    }
    pub fn set_bordered(&self, bordered: bool) -> Result<(), UiError> {
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setBordered:"), ((bordered): bool) => ())
        })
    }
    pub fn set_draws_background(&self, draws_background: bool) -> Result<(), UiError> {
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setDrawsBackground:"), ((draws_background): bool) => ())
        })
    }
    pub fn set_multiline(&self, multiline: bool) -> Result<(), UiError> {
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSTextField whose cell accepts text layout selectors.
            unsafe {
                let cell = zpd_objc::msg_send!(field, zpd_objc::sel!("cell"), () => zpd_objc::Id);
                let line_break_mode = if multiline {
                    native::NS_LINE_BREAK_BY_WORD_WRAPPING
                } else {
                    native::NS_LINE_BREAK_BY_CLIPPING
                };
                zpd_objc::msg_send!(field, zpd_objc::sel!("setUsesSingleLineMode:"), ((!multiline): bool) => ());
                zpd_objc::msg_send!(field, zpd_objc::sel!("setLineBreakMode:"), ((line_break_mode): u64) => ());
                zpd_objc::msg_send!(cell, zpd_objc::sel!("setScrollable:"), ((!multiline): bool) => ());
                zpd_objc::msg_send!(cell, zpd_objc::sel!("setWraps:"), ((multiline): bool) => ());
                zpd_objc::msg_send!(field, zpd_objc::sel!("setMaximumNumberOfLines:"), ((if multiline { 0 } else { 1 }): isize) => ());
            }
        })
    }
    /// Sets the font; `None` restores the standard system font.
    pub fn set_font(&self, font: Option<&Font>) -> Result<(), UiError> {
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSTextField; `font` keeps its NSFont alive for the call.
            unsafe {
                let font = match font {
                    Some(font) => font.as_ptr(),
                    None => {
                        let size = zpd_objc::msg_send!(zpd_objc::class!("NSFont"), zpd_objc::sel!("systemFontSize"), () => f64);
                        zpd_objc::msg_send!(zpd_objc::class!("NSFont"), zpd_objc::sel!("systemFontOfSize:"), ((size): f64) => zpd_objc::Id)
                    }
                };
                zpd_objc::msg_send!(field, zpd_objc::sel!("setFont:"), ((font): zpd_objc::Id) => ());
            }
        })
    }
    pub fn set_text_color(&self, color: TextColor) -> Result<(), UiError> {
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSTextField and the color is a live NSColor.
            unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setTextColor:"), ((color.native()): zpd_objc::Id) => ());
            }
        })
    }
    pub fn set_alignment(&self, alignment: TextAlignment) -> Result<(), UiError> {
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSTextField and the value is a valid NSTextAlignment.
            unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setAlignment:"), ((alignment.native()): isize) => ());
            }
        })
    }
    /// Wraps up to `lines` lines and truncates the last one with an ellipsis.
    ///
    /// Call after [`TextField::set_multiline`], which this overrides.
    pub fn set_line_limit(&self, lines: usize) -> Result<(), UiError> {
        let lines = lines.max(1);
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSTextField whose cell accepts text layout selectors.
            unsafe {
                let cell = zpd_objc::msg_send!(field, zpd_objc::sel!("cell"), () => zpd_objc::Id);
                zpd_objc::msg_send!(field, zpd_objc::sel!("setUsesSingleLineMode:"), ((lines == 1): bool) => ());
                zpd_objc::msg_send!(cell, zpd_objc::sel!("setWraps:"), ((lines > 1): bool) => ());
                zpd_objc::msg_send!(cell, zpd_objc::sel!("setScrollable:"), ((false): bool) => ());
                zpd_objc::msg_send!(cell, zpd_objc::sel!("setTruncatesLastVisibleLine:"), ((true): bool) => ());
                zpd_objc::msg_send!(field, zpd_objc::sel!("setLineBreakMode:"), ((native::NS_LINE_BREAK_BY_TRUNCATING_TAIL): u64) => ());
                zpd_objc::msg_send!(field, zpd_objc::sel!("setMaximumNumberOfLines:"), ((lines as isize): isize) => ());
            }
        })
    }
}
impl AsView for TextField {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}
