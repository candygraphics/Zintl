use std::cell::Cell;
use std::rc::Rc;

use crate::actor::WindowEventKind;
use crate::native;
use crate::runloop::Application;
use zpd_objc::Strong;

use super::UiError;
use super::callback;
use super::view::{AsView, ViewActor, ViewRef};

pub use crate::model::{BezelStyle, ButtonType};

impl ButtonType {
    fn is_stateful(self) -> bool {
        self != Self::MomentaryPushIn
    }
}

pub struct Button {
    view: ViewActor,
    stateful: Rc<Cell<bool>>,
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
        callback::install_action(view.actor(), move |button| {
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
        Ok(Self { view, stateful })
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
