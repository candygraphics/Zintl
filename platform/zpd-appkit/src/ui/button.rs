use std::cell::Cell;
use std::rc::Rc;

use crate::actor::WindowEventKind;
use crate::native;
use crate::runloop::Application;
use zpd_objc::Strong;

use super::UiError;
use super::callback;
use super::view::{AsView, ViewActor, ViewRef};

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
}
impl Button {
    pub fn with_title(application: &Application, title: &str) -> Result<Self, UiError> {
        let title = native::nsstring(title);
        // SAFETY: NSButton's title constructor returns an autoreleased live object, retained here.
        let native = unsafe {
            let value = zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSButton"), zpd_objc::sel!("alloc"), () => zpd_objc::Id), zpd_objc::sel!("init"), () => zpd_objc::Id);
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
        })
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
