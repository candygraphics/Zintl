use crate::actor::WindowEventKind;
use crate::graphics::Font;
use crate::native;
use crate::runloop::Application;
use zpd_objc::Strong;

use super::callback;
use super::field_accessory::{AccessorySlot, FieldAccessory};
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
    /// A dynamic system color, resolved when the text is drawn.
    System(SystemColor),
    Custom(Color),
}

/// A dynamic `NSColor.system…` color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Gray,
}

impl TextColor {
    /// Returns an autoreleased `NSColor`.
    fn native(self) -> zpd_objc::Id {
        let name = match self {
            Self::Label => zpd_objc::sel!("labelColor"),
            Self::SecondaryLabel => zpd_objc::sel!("secondaryLabelColor"),
            Self::TertiaryLabel => zpd_objc::sel!("tertiaryLabelColor"),
            Self::Accent => zpd_objc::sel!("controlAccentColor"),
            Self::System(SystemColor::Red) => zpd_objc::sel!("systemRedColor"),
            Self::System(SystemColor::Orange) => zpd_objc::sel!("systemOrangeColor"),
            Self::System(SystemColor::Yellow) => zpd_objc::sel!("systemYellowColor"),
            Self::System(SystemColor::Green) => zpd_objc::sel!("systemGreenColor"),
            Self::System(SystemColor::Blue) => zpd_objc::sel!("systemBlueColor"),
            Self::System(SystemColor::Gray) => zpd_objc::sel!("systemGrayColor"),
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
    pub(crate) fn native(self) -> isize {
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
    accessory: AccessorySlot,
    /// Whether the cell's accessibility role was written, so it is kept up to date.
    sets_role: std::cell::Cell<bool>,
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
            let target = callback::text_target(move |notification| unsafe {
                let field =
                    zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => zpd_objc::Id);
                let value = native::rust_string(
                    zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => zpd_objc::Id),
                );
                let kind = if callback::is_end_editing(notification) {
                    WindowEventKind::TextCommitted { value }
                } else {
                    WindowEventKind::TextChanged { value }
                };
                if let Some(tree) = actor.tree_handle() {
                    tree.emit(&actor, kind);
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
        Ok(Self {
            view,
            accessory: AccessorySlot::default(),
            sets_role: std::cell::Cell::new(false),
        })
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
    /// Allows or prevents typing; a field with a border then stays a text field to accessibility.
    pub fn set_editable(&self, editable: bool) -> Result<(), UiError> {
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setEditable:"), ((editable): bool) => ())
        })?;
        self.update_accessibility_role()
    }
    pub fn set_selectable(&self, selectable: bool) -> Result<(), UiError> {
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setSelectable:"), ((selectable): bool) => ())
        })
    }
    pub fn set_bordered(&self, bordered: bool) -> Result<(), UiError> {
        self.view.as_view().with(|field| unsafe {
            zpd_objc::msg_send!(field, zpd_objc::sel!("setBordered:"), ((bordered): bool) => ())
        })?;
        self.update_accessibility_role()
    }
    /// Reports a read-only field with a border as a text field rather than static text.
    ///
    /// AppKit reports every non-editable field as static text. Once written,
    /// the role follows later changes, since AppKit cannot restore its own.
    fn update_accessibility_role(&self) -> Result<(), UiError> {
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSTextField whose cell implements NSAccessibility and
            // copies the role string.
            unsafe {
                let editable = zpd_objc::msg_send!(field, zpd_objc::sel!("isEditable"), () => bool);
                let framed = zpd_objc::msg_send!(field, zpd_objc::sel!("isBezeled"), () => bool)
                    || zpd_objc::msg_send!(field, zpd_objc::sel!("isBordered"), () => bool);
                let read_only = !editable && framed;
                if !read_only && !self.sets_role.get() {
                    return;
                }
                self.sets_role.set(true);
                let role = if editable || framed { "AXTextField" } else { "AXStaticText" };
                let cell = zpd_objc::msg_send!(field, zpd_objc::sel!("cell"), () => zpd_objc::Id);
                zpd_objc::msg_send!(cell, zpd_objc::sel!("setAccessibilityRole:"), ((native::nsstring(role).as_ptr()): zpd_objc::Id) => ());
            }
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
    /// Makes the field the first responder of its window; does nothing outside a window.
    /// Shows `accessory` inside the field's trailing edge, or removes it for `None`.
    ///
    /// Clicks are reported as [`WindowEventKind::AccessoryClicked`].
    pub fn set_accessory(&self, accessory: Option<&FieldAccessory>) -> Result<(), UiError> {
        let actor = self.view.actor();
        self.accessory.set(actor, actor, accessory)
    }
    pub fn focus(&self) -> Result<(), UiError> {
        self.view.as_view().with(|field| {
            // SAFETY: The live field and its window, which may be nil, accept the message.
            unsafe {
                let window = zpd_objc::msg_send!(field, zpd_objc::sel!("window"), () => zpd_objc::Id);
                if !window.is_null() {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("makeFirstResponder:"), ((field): zpd_objc::Id) => bool);
                }
            }
        })
    }
    /// Returns the size the text needs at `width`, or its natural size for `None`.
    ///
    /// Wrapping fields grow in height as `width` shrinks.
    pub fn fitting_size(&self, width: Option<f64>) -> Result<(f64, f64), UiError> {
        const UNBOUNDED: f64 = 1.0e7;
        let bounds = native::Rect {
            origin: native::Point::default(),
            size: native::Size {
                width: width.unwrap_or(UNBOUNDED),
                height: UNBOUNDED,
            },
        };
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSTextField; its cell measures text within plain bounds.
            unsafe {
                let cell = zpd_objc::msg_send!(field, zpd_objc::sel!("cell"), () => zpd_objc::Id);
                let size = zpd_objc::msg_send!(cell, zpd_objc::sel!("cellSizeForBounds:"), ((bounds): native::Rect) => native::Size);
                (size.width.ceil(), size.height.ceil())
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
