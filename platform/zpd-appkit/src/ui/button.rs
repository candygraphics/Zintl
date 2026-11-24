use std::cell::Cell;
use std::rc::Rc;
use std::sync::Mutex;

use crate::actor::WindowEventKind;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, Sel, Strong};

use super::callback;
use super::view::{AsView, ViewActor, ViewRef};
use super::{Color, UiError};

/// Mirrors `NSButtonType` values used by declarative buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u64)]
pub enum ButtonType {
    #[default]
    MomentaryPushIn = 7,
    PushOnPushOff = 1,
    Checkbox = 3,
    Radio = 4,
}

impl ButtonType {
    fn is_stateful(self) -> bool {
        self != Self::MomentaryPushIn
    }
}

/// Mirrors `NSBezelStyle` values that are valid on current macOS releases.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u64)]
pub enum BezelStyle {
    #[default]
    Push = 1,
    FlexiblePush = 2,
    Disclosure = 5,
    Circular = 7,
    Help = 9,
    SmallSquare = 10,
    Toolbar = 11,
    AccessoryBarAction = 12,
    AccessoryBar = 13,
    PushDisclosure = 14,
    Badge = 15,
}

pub struct Button {
    view: ViewActor,
    stateful: Rc<Cell<bool>>,
    shows_menu: Rc<Cell<bool>>,
    keeps_state: Rc<Cell<bool>>,
    /// Kept so that a new title gets the same color.
    title_color: Cell<Option<Color>>,
}
impl Button {
    pub fn with_title(application: &Application, title: &str) -> Result<Self, UiError> {
        let title = native::nsstring(title);
        // SAFETY: NSButton's title constructor returns an autoreleased live object, retained here.
        let native = unsafe {
            let value = zpd_objc::msg_send!(zpd_objc::msg_send!(Self::key_class(zpd_objc::class!("NSButton")), zpd_objc::sel!("alloc"), () => zpd_objc::Id), zpd_objc::sel!("init"), () => zpd_objc::Id);
            let value = Strong::from_retained(value).ok_or(UiError::NativeCreationFailed)?;
            zpd_objc::msg_send!(value.as_ptr(), zpd_objc::sel!("setTitle:"), ((title.as_ptr()): zpd_objc::Id) => ());
            zpd_objc::msg_send!(value.as_ptr(), zpd_objc::sel!("setBezelStyle:"), ((BezelStyle::Push as u64): u64) => ());
            value
        };
        let view = ViewActor::new(application, native);
        let stateful = Rc::new(Cell::new(false));
        let emit = callback::emitter(view.actor());
        let emits_state = stateful.clone();
        let shows_menu = Rc::new(Cell::new(false));
        let pops_up = shows_menu.clone();
        let keeps_state = Rc::new(Cell::new(false));
        let restores = keeps_state.clone();
        callback::install_action(view.actor(), move |button| {
            if pops_up.get() {
                // SAFETY: The action sender is the live NSButton; its menu is nil or live.
                unsafe { Self::pop_up_menu(button) };
                return;
            }
            if restores.get() {
                // SAFETY: The action sender is the live NSButton; the click toggled its state.
                unsafe {
                    let state =
                        zpd_objc::msg_send!(button, zpd_objc::sel!("state"), () => native::Integer);
                    let previous = if state == native::NS_CONTROL_STATE_VALUE_ON {
                        0
                    } else {
                        native::NS_CONTROL_STATE_VALUE_ON
                    };
                    zpd_objc::msg_send!(button, zpd_objc::sel!("setState:"), ((previous): native::Integer) => ());
                }
                emit(WindowEventKind::ButtonClicked);
                return;
            }
            emit(WindowEventKind::ButtonClicked);
            if emits_state.get() {
                // SAFETY: The action sender is the live NSButton.
                let state = unsafe {
                    zpd_objc::msg_send!(button, zpd_objc::sel!("state"), () => native::Integer)
                };
                emit(WindowEventKind::StateChanged {
                    on: state == native::NS_CONTROL_STATE_VALUE_ON,
                });
            }
        })?;
        Ok(Self {
            view,
            stateful,
            shows_menu,
            keeps_state,
            title_color: Cell::new(None),
        })
    }
    /// Returns a subclass of `base` without ivars whose key equivalent leaves Return and
    /// Escape to an input method while it is composing text.
    ///
    /// Subclasses are created once per base class, so AppKit's own button subclasses, such
    /// as the ones `NSAlert` creates, keep their behavior.
    ///
    /// # Safety
    /// `base` must be `NSButton` or one of its subclasses.
    pub(crate) unsafe fn key_class(base: zpd_objc::Class) -> zpd_objc::Class {
        let mut classes = Self::key_classes()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some((_, class)) = classes
            .iter()
            .find(|(from, class)| *from == base as usize || *class == base as usize)
        {
            return *class as zpd_objc::Class;
        }
        // SAFETY: Guaranteed by the caller; the new class is named after its base, so names are
        // unique, and it is registered once with only an override of an NSButton method.
        let class = unsafe {
            let name = std::ffi::CStr::from_ptr(zpd_objc::ffi::class_getName(base));
            let name = std::ffi::CString::new(format!("ZpdKey_{}", name.to_string_lossy()))
                .expect("class names have no NUL");
            let class = zpd_objc::ffi::objc_allocateClassPair(base, name.as_ptr(), 0);
            assert!(!class.is_null(), "failed to allocate a key button class");
            let implementation = Self::perform_key_equivalent as *const ();
            let added = zpd_objc::ffi::class_addMethod(
                class,
                zpd_objc::sel!("performKeyEquivalent:"),
                std::mem::transmute::<*const (), zpd_objc::ffi::Imp>(implementation),
                c"B@:@".as_ptr(),
            );
            assert!(added, "failed to add performKeyEquivalent:");
            zpd_objc::ffi::objc_registerClassPair(class);
            class
        };
        classes.push((base as usize, class as usize));
        class
    }

    /// Pairs of a base class and its subclass from [`Button::key_class`].
    fn key_classes() -> &'static Mutex<Vec<(usize, usize)>> {
        static CLASSES: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
        &CLASSES
    }

    /// Returns the base class of the [`Button::key_class`] subclass that `object` is an instance of.
    ///
    /// # Safety
    /// `object` must be a live instance of a [`Button::key_class`] subclass.
    unsafe fn key_base(object: Id) -> zpd_objc::Class {
        let classes = Self::key_classes()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // SAFETY: Guaranteed by the caller; the chain ends at a root class whose superclass is nil.
        let mut class = unsafe { zpd_objc::ffi::object_getClass(object) };
        while !class.is_null() {
            if let Some((base, _)) = classes.iter().find(|(_, key)| *key == class as usize) {
                return *base as zpd_objc::Class;
            }
            // SAFETY: `class` is a live, registered class.
            class = unsafe { zpd_objc::ffi::class_getSuperclass(class) };
        }
        zpd_objc::class!("NSButton")
    }

    /// Returns `NO` for Return, Enter, and Escape while the window's first responder has
    /// unconfirmed text, so those keys confirm or cancel the conversion instead.
    unsafe extern "C" fn perform_key_equivalent(this: Id, _: Sel, event: Id) -> bool {
        zpd_objc::abort_on_unwind(|| {
            // SAFETY: `this` is a live key button and `event` a live NSEvent; the window and its
            // first responder are nil or live.
            unsafe {
                if Self::is_composition_key(event) && Self::window_is_composing(this) {
                    return false;
                }
                let base = Self::key_base(this);
                zpd_objc::msg_send_super!(this, base, zpd_objc::sel!("performKeyEquivalent:"), ((event): Id) => bool)
            }
        })
    }

    /// Returns whether `event` is a key an input method uses to confirm or cancel text.
    ///
    /// # Safety
    /// `event` must be a live NSEvent.
    unsafe fn is_composition_key(event: Id) -> bool {
        // SAFETY: Guaranteed by the caller; non-key events return nil characters.
        unsafe {
            if zpd_objc::msg_send!(event, zpd_objc::sel!("type"), () => native::Integer)
                != native::NS_EVENT_TYPE_KEY_DOWN
            {
                return false;
            }
            let characters =
                zpd_objc::msg_send!(event, zpd_objc::sel!("charactersIgnoringModifiers"), () => Id);
            !characters.is_null()
                && matches!(
                    native::rust_string(characters).as_str(),
                    "\r" | "\u{3}" | "\u{1b}"
                )
        }
    }

    /// Returns whether the first responder of `view`'s window has unconfirmed (marked) text.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    unsafe fn window_is_composing(view: Id) -> bool {
        // SAFETY: Guaranteed by the caller; `hasMarkedText` is sent only to responders that
        // implement it, such as the field editor.
        unsafe {
            let window = zpd_objc::msg_send!(view, zpd_objc::sel!("window"), () => Id);
            if window.is_null() {
                return false;
            }
            let responder = zpd_objc::msg_send!(window, zpd_objc::sel!("firstResponder"), () => Id);
            !responder.is_null()
                && zpd_objc::msg_send!(responder, zpd_objc::sel!("respondsToSelector:"), ((zpd_objc::sel!("hasMarkedText")): Sel) => bool)
                && zpd_objc::msg_send!(responder, zpd_objc::sel!("hasMarkedText"), () => bool)
        }
    }

    /// Switches `button`, such as one an `NSAlert` created, to its [`Button::key_class`].
    ///
    /// # Safety
    /// `button` must be a live `NSButton`.
    pub(crate) unsafe fn use_key_class(button: Id) {
        // SAFETY: Guaranteed by the caller; the subclass adds no ivars, so the layout is kept.
        // KVO subclasses are left alone, since KVO expects to own the object's class.
        unsafe {
            let current = zpd_objc::ffi::object_getClass(button);
            let name = std::ffi::CStr::from_ptr(zpd_objc::ffi::class_getName(current));
            if !name.to_bytes().starts_with(b"NSKVONotifying_") {
                zpd_objc::ffi::object_setClass(button, Self::key_class(current));
            }
        }
    }
    /// Undoes the toggle of each click, leaving the state to [`Button::set_on`].
    pub fn set_keeps_state(&self, keeps: bool) {
        self.keeps_state.set(keeps);
    }
    /// Makes a click show the button's menu below it instead of emitting `ButtonClicked`.
    pub fn set_shows_menu(&self, shows: bool) {
        self.shows_menu.set(shows);
    }
    /// Shows the menu of `button` below its bottom-left corner.
    ///
    /// # Safety
    /// `button` must be a live NSButton.
    unsafe fn pop_up_menu(button: zpd_objc::Id) {
        // SAFETY: Guaranteed by the caller; a nil menu is skipped.
        unsafe {
            let menu = zpd_objc::msg_send!(button, zpd_objc::sel!("menu"), () => zpd_objc::Id);
            if menu.is_null() {
                return;
            }
            let bounds = zpd_objc::msg_send!(button, zpd_objc::sel!("bounds"), () => native::Rect);
            let flipped = zpd_objc::msg_send!(button, zpd_objc::sel!("isFlipped"), () => bool);
            let y = if flipped {
                bounds.origin.y + bounds.size.height + 4.0
            } else {
                bounds.origin.y - 4.0
            };
            let point = native::Point {
                x: bounds.origin.x,
                y,
            };
            zpd_objc::msg_send!(menu, zpd_objc::sel!("popUpMenuPositioningItem:atLocation:inView:"), ((zpd_objc::NIL): zpd_objc::Id, (point): native::Point, (button): zpd_objc::Id) => bool);
        }
    }
    pub fn set_title(&self, title: &str) -> Result<(), UiError> {
        let title = native::nsstring(title);
        self.view.as_view().with(|button| unsafe {
            zpd_objc::msg_send!(button, zpd_objc::sel!("setTitle:"), ((title.as_ptr()): zpd_objc::Id) => ())
        })?;
        self.apply_title_color()
    }

    /// Draws the title in `color`, or in the system's color for `None`.
    pub fn set_title_color(&self, color: Option<Color>) -> Result<(), UiError> {
        self.title_color.set(color);
        if color.is_some() {
            return self.apply_title_color();
        }
        // A plain title replaces the colored attributed one.
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton; its title is a live NSString.
            unsafe {
                let title = zpd_objc::msg_send!(button, zpd_objc::sel!("title"), () => zpd_objc::Id);
                zpd_objc::msg_send!(button, zpd_objc::sel!("setTitle:"), ((title): zpd_objc::Id) => ());
            }
        })
    }

    /// Colors the current title, keeping the font and alignment AppKit chose for it.
    fn apply_title_color(&self) -> Result<(), UiError> {
        let Some(color) = self.title_color.get() else {
            return Ok(());
        };
        let key = native::nsstring("NSColor");
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton; the mutable copy is owned and released here.
            unsafe {
                let title = zpd_objc::msg_send!(button, zpd_objc::sel!("attributedTitle"), () => zpd_objc::Id);
                let colored = zpd_objc::msg_send!(title, zpd_objc::sel!("mutableCopy"), () => zpd_objc::Id);
                let length = zpd_objc::msg_send!(colored, zpd_objc::sel!("length"), () => u64);
                let range = native::Range {
                    location: 0,
                    length,
                };
                zpd_objc::msg_send!(colored, zpd_objc::sel!("addAttribute:value:range:"), ((key.as_ptr()): zpd_objc::Id, (color.native()): zpd_objc::Id, (range): native::Range) => ());
                zpd_objc::msg_send!(button, zpd_objc::sel!("setAttributedTitle:"), ((colored): zpd_objc::Id) => ());
                zpd_objc::msg_send!(colored, zpd_objc::sel!("release"), () => ());
            }
        })
    }
    /// Selects momentary, toggle, checkbox, or radio behavior.
    pub fn set_button_type(&self, button_type: ButtonType) -> Result<(), UiError> {
        self.stateful.set(button_type.is_stateful());
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton and setButtonType: takes an NSButtonType.
            unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("setButtonType:"), ((button_type as u64): u64) => ())
            }
        })
    }
    pub fn set_bezel_style(&self, style: BezelStyle) -> Result<(), UiError> {
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton and setBezelStyle: takes an NSBezelStyle.
            unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("setBezelStyle:"), ((style as u64): u64) => ())
            }
        })
    }
    pub fn set_bordered(&self, bordered: bool) -> Result<(), UiError> {
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton.
            unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("setBordered:"), ((bordered): bool) => ())
            }
        })
    }
    pub fn set_on(&self, on: bool) -> Result<(), UiError> {
        let state = if on {
            native::NS_CONTROL_STATE_VALUE_ON
        } else {
            0
        };
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton and setState: takes an NSControlStateValue.
            unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("setState:"), ((state): native::Integer) => ())
            }
        })
    }
    pub fn is_on(&self) -> Result<bool, UiError> {
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton.
            let state = unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("state"), () => native::Integer)
            };
            state == native::NS_CONTROL_STATE_VALUE_ON
        })
    }
    /// Shows an SF Symbol before the title, or alone when the title is empty.
    pub fn set_system_image(&self, name: Option<&str>) -> Result<(), UiError> {
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton; nil clears its image.
            unsafe {
                let title = native::rust_string(
                    zpd_objc::msg_send!(button, zpd_objc::sel!("title"), () => zpd_objc::Id),
                );
                let image = name.map_or(zpd_objc::NIL, |name| native::symbol_image(name, &title));
                let position = if title.is_empty() {
                    native::NS_IMAGE_ONLY
                } else {
                    native::NS_IMAGE_LEADING
                };
                zpd_objc::msg_send!(button, zpd_objc::sel!("setImage:"), ((image): zpd_objc::Id) => ());
                zpd_objc::msg_send!(button, zpd_objc::sel!("setImagePosition:"), ((position): u64) => ());
            }
        })
    }
    /// Sets the key that activates the button, such as `"\r"` or `"\u{1b}"`.
    pub fn set_key_equivalent(&self, key: &str) -> Result<(), UiError> {
        let key = native::nsstring(key);
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton and copies the NSString.
            unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("setKeyEquivalent:"), ((key.as_ptr()): zpd_objc::Id) => ())
            }
        })
    }
    /// Marks the button as performing a destructive action.
    pub fn set_destructive(&self, destructive: bool) -> Result<(), UiError> {
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSButton available on macOS 11 and later.
            unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("setHasDestructiveAction:"), ((destructive): bool) => ())
            }
        })
    }
}
impl AsView for Button {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}
