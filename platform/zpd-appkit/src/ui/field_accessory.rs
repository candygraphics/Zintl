//! A button inside the trailing edge of a text field, like a search field's cancel button.

use std::cell::RefCell;
use std::sync::OnceLock;

use zpd_objc::{Id, Sel};

use crate::actor::{ActorRef, WindowEventKind};
use crate::native;

use super::UiError;
use super::callback;
use super::view::ViewRef;

/// An SF Symbol button shown inside a text field's trailing edge.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldAccessory {
    /// The SF Symbol name.
    pub system_image: String,
    /// The accessibility label and tooltip.
    pub label: String,
    pub identifier: Option<String>,
}

/// The width the button takes from the trailing edge of the text area.
const WIDTH: f64 = 22.0;

/// Returns the text area of a field with an accessory: the standard one without the button's width.
unsafe extern "C" fn drawing_rect(this: Id, _: Sel, bounds: native::Rect) -> native::Rect {
    // SAFETY: `this` is an NSTextFieldCell whose class was swapped to this subclass.
    let mut rect = unsafe {
        zpd_objc::msg_send_super!(this, zpd_objc::class!("NSTextFieldCell"), zpd_objc::sel!("drawingRectForBounds:"), ((bounds): native::Rect) => native::Rect)
    };
    rect.size.width = (rect.size.width - WIDTH).max(0.0);
    rect
}

/// Returns the field's accessibility children followed by its visible accessory button.
///
/// AppKit does not list a text field's subviews, so VoiceOver could not reach the button.
unsafe extern "C" fn accessibility_children(this: Id, _: Sel) -> Id {
    // SAFETY: `this` is a live NSTextField whose class was swapped to this subclass; the arrays
    // are autoreleased and only read here.
    unsafe {
        let inherited = zpd_objc::msg_send_super!(this, zpd_objc::class!("NSTextField"), zpd_objc::sel!("accessibilityChildren"), () => Id);
        let children = zpd_objc::msg_send!(zpd_objc::class!("NSMutableArray"), zpd_objc::sel!("array"), () => Id);
        if !inherited.is_null() {
            zpd_objc::msg_send!(children, zpd_objc::sel!("addObjectsFromArray:"), ((inherited): Id) => ());
        }
        let subviews = zpd_objc::msg_send!(this, zpd_objc::sel!("subviews"), () => Id);
        let count = zpd_objc::msg_send!(subviews, zpd_objc::sel!("count"), () => u64);
        for index in 0..count {
            let view = zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((index): u64) => Id);
            let button = zpd_objc::msg_send!(view, zpd_objc::sel!("isKindOfClass:"), ((zpd_objc::class!("NSButton")): zpd_objc::Class) => bool);
            let hidden = zpd_objc::msg_send!(view, zpd_objc::sel!("isHidden"), () => bool);
            if button && !hidden {
                zpd_objc::msg_send!(children, zpd_objc::sel!("addObject:"), ((view): Id) => ());
            }
        }
        children
    }
}

/// An `NSTextField` subclass without ivars, so a field can switch to it.
fn accessory_field_class() -> zpd_objc::Class {
    static CLASS: OnceLock<usize> = OnceLock::new();
    *CLASS.get_or_init(|| {
        zpd_objc::decl!(ZpdAccessoryTextField: [zpd_objc::class!("NSTextField")] {
            methods {
                "accessibilityChildren": "@@:" => accessibility_children,
            }
        }) as usize
    }) as zpd_objc::Class
}

/// Switches `object` between `standard` and its ivar-free subclass `accessory`.
///
/// Objects of other classes, such as KVO subclasses, are left alone.
///
/// # Safety
///
/// `object` must be live and `accessory` must add no ivars to `standard`.
unsafe fn switch_class(
    object: Id,
    standard: zpd_objc::Class,
    accessory: zpd_objc::Class,
    to_accessory: bool,
) {
    // SAFETY: The caller guarantees a live object and an ivar-free subclass.
    unsafe {
        let current = zpd_objc::ffi::object_getClass(object);
        let target = if to_accessory { accessory } else { standard };
        if (current == standard || current == accessory) && current != target {
            zpd_objc::ffi::object_setClass(object, target);
        }
    }
}

/// An `NSTextFieldCell` subclass without ivars, so a field's own cell can switch to it.
fn accessory_cell_class() -> zpd_objc::Class {
    static CLASS: OnceLock<usize> = OnceLock::new();
    *CLASS.get_or_init(|| {
        zpd_objc::decl!(ZpdAccessoryTextFieldCell: [zpd_objc::class!("NSTextFieldCell")] {
            methods {
                "drawingRectForBounds:": "{CGRect={CGPoint=dd}{CGSize=dd}}@:{CGRect={CGPoint=dd}{CGSize=dd}}" => drawing_rect,
            }
        }) as usize
    }) as zpd_objc::Class
}

/// The accessory button of one text field, created when first shown and hidden when removed.
#[derive(Default)]
pub(crate) struct AccessorySlot {
    button: RefCell<Option<ActorRef>>,
}

impl AccessorySlot {
    /// Shows `accessory` inside `field`, or hides it for `None`.
    ///
    /// Clicks are reported as [`WindowEventKind::AccessoryClicked`] from `owner`.
    pub(crate) fn set(
        &self,
        field: &ActorRef,
        owner: &ActorRef,
        accessory: Option<&FieldAccessory>,
    ) -> Result<(), UiError> {
        let shown = accessory.is_some();
        Self::inset_text(field, shown)?;
        let Some(accessory) = accessory else {
            if let Some(button) = self.button.borrow().as_ref() {
                ViewRef::from_actor(button).set_hidden(true)?;
            }
            return Ok(());
        };
        if self.button.borrow().is_none() {
            let button = Self::create_button(field, owner)?;
            *self.button.borrow_mut() = Some(button);
        }
        let button = self.button.borrow();
        let button = button.as_ref().expect("the button was created above");
        let label = native::nsstring(&accessory.label);
        button.with(|button| {
            // SAFETY: The live button copies the image and label; nil clears the image.
            unsafe {
                let image = native::symbol_image(&accessory.system_image, &accessory.label);
                zpd_objc::msg_send!(button, zpd_objc::sel!("setImage:"), ((image): Id) => ());
                zpd_objc::msg_send!(button, zpd_objc::sel!("setAccessibilityLabel:"), ((label.as_ptr()): Id) => ());
            }
        })?;
        let view = ViewRef::from_actor(button);
        view.set_tool_tip(Some(&accessory.label))?;
        view.set_identifier(accessory.identifier.as_deref())?;
        view.set_hidden(false)?;
        Ok(())
    }

    /// Switches the field and its cell between the standard classes and subclasses that
    /// leave room for the button and list it for accessibility.
    fn inset_text(field: &ActorRef, inset: bool) -> Result<(), UiError> {
        field.with(|field| {
            // SAFETY: The live field and its cell switch only between a standard class and its
            // ivar-free subclass, which keeps their object layout.
            unsafe {
                let cell = zpd_objc::msg_send!(field, zpd_objc::sel!("cell"), () => Id);
                switch_class(
                    field,
                    zpd_objc::class!("NSTextField"),
                    accessory_field_class(),
                    inset,
                );
                switch_class(
                    cell,
                    zpd_objc::class!("NSTextFieldCell"),
                    accessory_cell_class(),
                    inset,
                );
                zpd_objc::msg_send!(field, zpd_objc::sel!("setNeedsDisplay:"), ((true): bool) => ());
            }
        })?;
        Ok(())
    }

    /// Adds a borderless image button at the trailing edge of `field`.
    fn create_button(field: &ActorRef, owner: &ActorRef) -> Result<ActorRef, UiError> {
        let native = native::alloc_init(zpd_objc::class!("NSButton"));
        let frame = field.with(|field| {
            // SAFETY: The live field reports its bounds.
            unsafe { zpd_objc::msg_send!(field, zpd_objc::sel!("bounds"), () => native::Rect) }
        })?;
        let width = frame.size.width;
        let height = frame.size.height;
        // SAFETY: The new button and the live field are configured on the main thread.
        unsafe {
            let button = native.as_ptr();
            zpd_objc::msg_send!(button, zpd_objc::sel!("setBordered:"), ((false): bool) => ());
            zpd_objc::msg_send!(button, zpd_objc::sel!("setTitle:"), ((native::nsstring("").as_ptr()): Id) => ());
            zpd_objc::msg_send!(button, zpd_objc::sel!("setImagePosition:"), ((native::NS_IMAGE_ONLY): u64) => ());
            // Clicking it leaves keyboard focus and the selection in the field.
            zpd_objc::msg_send!(button, zpd_objc::sel!("setRefusesFirstResponder:"), ((true): bool) => ());
            let frame = native::Rect {
                // A flexible left margin keeps it at the trailing edge, even from a zero width.
                origin: native::Point {
                    x: width - WIDTH,
                    y: 0.0,
                },
                size: native::Size {
                    width: WIDTH,
                    height,
                },
            };
            zpd_objc::msg_send!(button, zpd_objc::sel!("setFrame:"), ((frame): native::Rect) => ());
            zpd_objc::msg_send!(button, zpd_objc::sel!("setAutoresizingMask:"), ((native::NS_VIEW_MIN_X_MARGIN | native::NS_VIEW_HEIGHT_SIZABLE): u64) => ());
        }
        field.with(|field| {
            // SAFETY: The live field retains the button as a subview.
            unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setAutoresizesSubviews:"), ((true): bool) => ());
                zpd_objc::msg_send!(field, zpd_objc::sel!("addSubview:"), ((native.as_ptr()): Id) => ());
            }
        })?;
        let tree = field.tree_handle().ok_or(UiError::Closed)?;
        let button = tree.insert_child(field, native)?;
        let emit = callback::emitter(owner);
        callback::install_action(&button, move |_| emit(WindowEventKind::AccessoryClicked))?;
        Ok(button)
    }
}
