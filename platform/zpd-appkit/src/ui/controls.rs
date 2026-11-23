//! Value controls that report semantic changes through the Actor tree.

use crate::actor::WindowEventKind;
use crate::geometry::Rect;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, Strong};

use super::callback;
use super::view::{AsView, ViewActor, ViewRef};
use super::{Color, UiError};

fn create(class: zpd_objc::Class) -> Result<Strong, UiError> {
    // SAFETY: Every caller passes an NSView subclass implementing alloc/init.
    unsafe {
        let object = zpd_objc::msg_send!(class, zpd_objc::sel!("alloc"), () => Id);
        Strong::from_retained(zpd_objc::msg_send!(object, zpd_objc::sel!("init"), () => Id))
    }
    .ok_or(UiError::NativeCreationFailed)
}

/// Reads `stringValue` from a live NSControl.
///
/// # Safety
/// `control` must be a live object implementing `stringValue`.
unsafe fn string_value(control: Id) -> String {
    // SAFETY: Guaranteed by the caller.
    unsafe {
        native::rust_string(zpd_objc::msg_send!(control, zpd_objc::sel!("stringValue"), () => Id))
    }
}

macro_rules! as_view {
    ($($control:ty),* $(,)?) => {
        $(impl AsView for $control {
            fn as_view(&self) -> ViewRef<'_> {
                self.view.as_view()
            }
        })*
    };
}

as_view!(
    Switch,
    Slider,
    PopUpButton,
    SegmentedControl,
    ComboBox,
    NumberField,
    SearchField,
    ColorWell,
    ProgressIndicator,
    Separator,
);

/// A native `NSProgressIndicator`, spinning or showing a fraction as a bar.
pub struct ProgressIndicator {
    view: ViewActor,
}
impl ProgressIndicator {
    const STYLE_BAR: u64 = 0;
    const STYLE_SPINNING: u64 = 1;

    pub fn new(application: &Application) -> Result<Self, UiError> {
        let native = create(zpd_objc::class!("NSProgressIndicator"))?;
        // SAFETY: The retained object is an NSProgressIndicator.
        unsafe {
            zpd_objc::msg_send!(native.as_ptr(), zpd_objc::sel!("setMinValue:"), ((0.0): f64) => ());
            zpd_objc::msg_send!(native.as_ptr(), zpd_objc::sel!("setMaxValue:"), ((1.0): f64) => ());
            zpd_objc::msg_send!(native.as_ptr(), zpd_objc::sel!("setDisplayedWhenStopped:"), ((true): bool) => ());
        }
        Ok(Self {
            view: ViewActor::new(application, native),
        })
    }
    /// Spins while `value` is `None`; otherwise shows `value` in `0.0..=1.0` as a bar.
    pub fn set_value(&self, value: Option<f64>) -> Result<(), UiError> {
        self.view.as_view().with(|indicator| {
            // SAFETY: The live view is an NSProgressIndicator.
            unsafe {
                match value {
                    None => {
                        zpd_objc::msg_send!(indicator, zpd_objc::sel!("setStyle:"), ((Self::STYLE_SPINNING): u64) => ());
                        zpd_objc::msg_send!(indicator, zpd_objc::sel!("setIndeterminate:"), ((true): bool) => ());
                        zpd_objc::msg_send!(indicator, zpd_objc::sel!("startAnimation:"), ((zpd_objc::NIL): Id) => ());
                    }
                    Some(value) => {
                        zpd_objc::msg_send!(indicator, zpd_objc::sel!("stopAnimation:"), ((zpd_objc::NIL): Id) => ());
                        zpd_objc::msg_send!(indicator, zpd_objc::sel!("setStyle:"), ((Self::STYLE_BAR): u64) => ());
                        zpd_objc::msg_send!(indicator, zpd_objc::sel!("setIndeterminate:"), ((false): bool) => ());
                        zpd_objc::msg_send!(indicator, zpd_objc::sel!("setDoubleValue:"), ((value.clamp(0.0, 1.0)): f64) => ());
                    }
                }
            }
        })
    }
}

/// A separator line drawn by an `NSBox`.
pub struct Separator {
    view: ViewActor,
}
impl Separator {
    const BOX_SEPARATOR: u64 = 2;

    pub fn new(application: &Application) -> Result<Self, UiError> {
        let native = create(zpd_objc::class!("NSBox"))?;
        // SAFETY: The retained object is an NSBox; separators draw a line along their longer side.
        unsafe {
            zpd_objc::msg_send!(native.as_ptr(), zpd_objc::sel!("setBoxType:"), ((Self::BOX_SEPARATOR): u64) => ());
        }
        Ok(Self {
            view: ViewActor::new(application, native),
        })
    }
}

/// A native `NSSwitch`.
pub struct Switch {
    view: ViewActor,
}
impl Switch {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        let view = ViewActor::new(application, create(zpd_objc::class!("NSSwitch"))?);
        let emit = callback::emitter(view.actor());
        callback::install_action(view.actor(), move |switch| {
            // SAFETY: The action sender is the live NSSwitch.
            let state = unsafe {
                zpd_objc::msg_send!(switch, zpd_objc::sel!("state"), () => native::Integer)
            };
            emit(WindowEventKind::StateChanged {
                on: state == native::NS_CONTROL_STATE_VALUE_ON,
            });
        })?;
        Ok(Self { view })
    }
    pub fn is_on(&self) -> Result<bool, UiError> {
        self.view.as_view().with(|switch| {
            // SAFETY: The live view is an NSSwitch.
            let state = unsafe {
                zpd_objc::msg_send!(switch, zpd_objc::sel!("state"), () => native::Integer)
            };
            state == native::NS_CONTROL_STATE_VALUE_ON
        })
    }
    pub fn set_on(&self, on: bool) -> Result<(), UiError> {
        let state = if on {
            native::NS_CONTROL_STATE_VALUE_ON
        } else {
            0
        };
        self.view.as_view().with(|switch| {
            // SAFETY: The live view is an NSSwitch.
            unsafe {
                zpd_objc::msg_send!(switch, zpd_objc::sel!("setState:"), ((state): native::Integer) => ())
            }
        })
    }
}

/// A continuous native `NSSlider`.
pub struct Slider {
    view: ViewActor,
}
impl Slider {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        let native = create(zpd_objc::class!("NSSlider"))?;
        // SAFETY: The retained object is an NSSlider.
        unsafe {
            zpd_objc::msg_send!(native.as_ptr(), zpd_objc::sel!("setContinuous:"), ((true): bool) => ())
        };
        let view = ViewActor::new(application, native);
        let emit = callback::emitter(view.actor());
        callback::install_action(view.actor(), move |slider| {
            // SAFETY: The action sender is the live NSSlider.
            let value =
                unsafe { zpd_objc::msg_send!(slider, zpd_objc::sel!("doubleValue"), () => f64) };
            emit(WindowEventKind::ValueChanged { value });
        })?;
        Ok(Self { view })
    }
    pub fn value(&self) -> Result<f64, UiError> {
        self.view.as_view().with(|slider| {
            // SAFETY: The live view is an NSSlider.
            unsafe { zpd_objc::msg_send!(slider, zpd_objc::sel!("doubleValue"), () => f64) }
        })
    }
    pub fn set_value(&self, value: f64) -> Result<(), UiError> {
        self.view.as_view().with(|slider| {
            // SAFETY: The live view is an NSSlider.
            unsafe {
                zpd_objc::msg_send!(slider, zpd_objc::sel!("setDoubleValue:"), ((value): f64) => ())
            }
        })
    }
    pub fn set_range(&self, minimum: f64, maximum: f64) -> Result<(), UiError> {
        self.view.as_view().with(|slider| {
            // SAFETY: The live view is an NSSlider.
            unsafe {
                zpd_objc::msg_send!(slider, zpd_objc::sel!("setMinValue:"), ((minimum): f64) => ());
                zpd_objc::msg_send!(slider, zpd_objc::sel!("setMaxValue:"), ((maximum): f64) => ());
            }
        })
    }
    /// Shows `count` tick marks; `snap` restricts values to them.
    pub fn set_tick_marks(&self, count: usize, snap: bool) -> Result<(), UiError> {
        self.view.as_view().with(|slider| {
            // SAFETY: The live view is an NSSlider.
            unsafe {
                zpd_objc::msg_send!(slider, zpd_objc::sel!("setNumberOfTickMarks:"), ((count as native::Integer): native::Integer) => ());
                zpd_objc::msg_send!(slider, zpd_objc::sel!("setAllowsTickMarkValuesOnly:"), ((snap && count > 1): bool) => ());
            }
        })
    }
}

/// Builds `NSMenuItem`s without the title de-duplication of `addItemWithTitle:`.
///
/// # Safety
/// `menu` must be a live `NSMenu`.
unsafe fn replace_menu_items(menu: Id, titles: &[String]) {
    // SAFETY: Guaranteed by the caller; each new item is retained by the menu.
    unsafe {
        zpd_objc::msg_send!(menu, zpd_objc::sel!("removeAllItems"), () => ());
        let empty = native::nsstring("");
        for title in titles {
            let title = native::nsstring(title);
            let allocated = zpd_objc::msg_send!(zpd_objc::class!("NSMenuItem"), zpd_objc::sel!("alloc"), () => Id);
            let item = zpd_objc::msg_send!(allocated, zpd_objc::sel!("initWithTitle:action:keyEquivalent:"), ((title.as_ptr()): Id, (zpd_objc::NIL): Id, (empty.as_ptr()): Id) => Id);
            if let Some(item) = Strong::from_retained(item) {
                zpd_objc::msg_send!(menu, zpd_objc::sel!("addItem:"), ((item.as_ptr()): Id) => ());
            }
        }
    }
}

/// A native `NSPopUpButton` that selects one item.
pub struct PopUpButton {
    view: ViewActor,
}
impl PopUpButton {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        // SAFETY: NSPopUpButton's designated initializer returns a retained object.
        let native = unsafe {
            let object = zpd_objc::msg_send!(zpd_objc::class!("NSPopUpButton"), zpd_objc::sel!("alloc"), () => Id);
            Strong::from_retained(zpd_objc::msg_send!(object, zpd_objc::sel!("initWithFrame:pullsDown:"), ((native::Rect::default()): native::Rect, (false): bool) => Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        let view = ViewActor::new(application, native);
        let emit = callback::emitter(view.actor());
        callback::install_action(view.actor(), move |button| {
            // SAFETY: The action sender is the live NSPopUpButton.
            let index = unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("indexOfSelectedItem"), () => native::Integer)
            };
            if let Ok(index) = usize::try_from(index) {
                emit(WindowEventKind::SelectionChanged { index });
            }
        })?;
        Ok(Self { view })
    }
    pub fn set_items(&self, titles: &[String]) -> Result<(), UiError> {
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSPopUpButton with a live menu.
            unsafe {
                replace_menu_items(
                    zpd_objc::msg_send!(button, zpd_objc::sel!("menu"), () => Id),
                    titles,
                )
            }
        })
    }
    pub fn selected(&self) -> Result<Option<usize>, UiError> {
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSPopUpButton.
            let index = unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("indexOfSelectedItem"), () => native::Integer)
            };
            usize::try_from(index).ok()
        })
    }
    pub fn select(&self, index: Option<usize>) -> Result<(), UiError> {
        let index = index.map_or(-1, |index| index as native::Integer);
        self.view.as_view().with(|button| {
            // SAFETY: The live view is an NSPopUpButton; -1 clears the selection.
            unsafe {
                zpd_objc::msg_send!(button, zpd_objc::sel!("selectItemAtIndex:"), ((index): native::Integer) => ())
            }
        })
    }
}

/// A native `NSSegmentedControl` that selects one segment.
pub struct SegmentedControl {
    view: ViewActor,
}
impl SegmentedControl {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        let native = create(zpd_objc::class!("NSSegmentedControl"))?;
        // SAFETY: The retained object is an NSSegmentedControl.
        unsafe {
            zpd_objc::msg_send!(native.as_ptr(), zpd_objc::sel!("setTrackingMode:"), ((native::NS_SEGMENT_SWITCH_TRACKING_SELECT_ONE): u64) => ());
            zpd_objc::msg_send!(native.as_ptr(), zpd_objc::sel!("setSegmentDistribution:"), ((native::NS_SEGMENT_DISTRIBUTION_FILL_EQUALLY): native::Integer) => ());
        }
        let view = ViewActor::new(application, native);
        let emit = callback::emitter(view.actor());
        callback::install_action(view.actor(), move |control| {
            // SAFETY: The action sender is the live NSSegmentedControl.
            let index = unsafe {
                zpd_objc::msg_send!(control, zpd_objc::sel!("selectedSegment"), () => native::Integer)
            };
            if let Ok(index) = usize::try_from(index) {
                emit(WindowEventKind::SelectionChanged { index });
            }
        })?;
        Ok(Self { view })
    }
    pub fn set_segments(&self, labels: &[String]) -> Result<(), UiError> {
        self.view.as_view().with(|control| {
            // SAFETY: The live view is an NSSegmentedControl; labels are copied.
            unsafe {
                zpd_objc::msg_send!(control, zpd_objc::sel!("setSegmentCount:"), ((labels.len() as native::Integer): native::Integer) => ());
                for (index, label) in labels.iter().enumerate() {
                    let label = native::nsstring(label);
                    zpd_objc::msg_send!(control, zpd_objc::sel!("setLabel:forSegment:"), ((label.as_ptr()): Id, (index as native::Integer): native::Integer) => ());
                }
            }
        })
    }
    pub fn selected(&self) -> Result<Option<usize>, UiError> {
        self.view.as_view().with(|control| {
            // SAFETY: The live view is an NSSegmentedControl.
            let index = unsafe {
                zpd_objc::msg_send!(control, zpd_objc::sel!("selectedSegment"), () => native::Integer)
            };
            usize::try_from(index).ok()
        })
    }
    pub fn select(&self, index: Option<usize>) -> Result<(), UiError> {
        let index = index.map_or(-1, |index| index as native::Integer);
        self.view.as_view().with(|control| {
            // SAFETY: The live view is an NSSegmentedControl; -1 clears the selection.
            unsafe {
                zpd_objc::msg_send!(control, zpd_objc::sel!("setSelectedSegment:"), ((index): native::Integer) => ())
            }
        })
    }
}

/// An editable native `NSComboBox` with suggested values.
pub struct ComboBox {
    view: ViewActor,
}
impl ComboBox {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        let native = create(zpd_objc::class!("NSComboBox"))?;
        // SAFETY: The retained object is an NSComboBox.
        unsafe {
            zpd_objc::msg_send!(native.as_ptr(), zpd_objc::sel!("setCompletes:"), ((true): bool) => ())
        };
        let view = ViewActor::new(application, native);
        let emit = callback::emitter(view.actor());
        callback::install_delegate(view.actor(), move |notification| {
            // SAFETY: Combo box notifications carry the live NSComboBox as their object.
            let value = unsafe {
                let combo = zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id);
                let name = native::rust_string(
                    zpd_objc::msg_send!(notification, zpd_objc::sel!("name"), () => Id),
                );
                if name == "NSComboBoxSelectionDidChangeNotification" {
                    let selected = zpd_objc::msg_send!(combo, zpd_objc::sel!("objectValueOfSelectedItem"), () => Id);
                    native::rust_string(selected)
                } else {
                    string_value(combo)
                }
            };
            emit(WindowEventKind::TextChanged { value });
        })?;
        Ok(Self { view })
    }
    pub fn set_items(&self, items: &[String]) -> Result<(), UiError> {
        let items = native::nsstring_array(items);
        self.view.as_view().with(|combo| {
            // SAFETY: The live view is an NSComboBox that copies the array contents.
            unsafe {
                zpd_objc::msg_send!(combo, zpd_objc::sel!("removeAllItems"), () => ());
                zpd_objc::msg_send!(combo, zpd_objc::sel!("addItemsWithObjectValues:"), ((items.as_ptr()): Id) => ());
            }
        })
    }
    pub fn string_value(&self) -> Result<String, UiError> {
        // SAFETY: The live view is an NSComboBox.
        self.view
            .as_view()
            .with(|combo| unsafe { string_value(combo) })
    }
    pub fn set_string_value(&self, value: &str) -> Result<(), UiError> {
        let value = native::nsstring(value);
        self.view.as_view().with(|combo| {
            // SAFETY: The live view is an NSComboBox.
            unsafe {
                zpd_objc::msg_send!(combo, zpd_objc::sel!("setStringValue:"), ((value.as_ptr()): Id) => ())
            }
        })
    }
    pub fn set_placeholder(&self, value: Option<&str>) -> Result<(), UiError> {
        let value = value.map(native::nsstring);
        self.view.as_view().with(|combo| {
            // SAFETY: The live view is an NSComboBox; nil clears the placeholder.
            unsafe {
                zpd_objc::msg_send!(combo, zpd_objc::sel!("setPlaceholderString:"), ((value.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): Id) => ())
            }
        })
    }
}

/// Numeric bounds and formatting for [`NumberField`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberFormat {
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
    pub fraction_digits: usize,
}

/// A number-only `NSTextField` paired with an `NSStepper`.
///
/// With [`NumberField::set_text`] the field shows caller-formatted text and
/// reports edits as [`WindowEventKind::TextChanged`] while the user types and
/// [`WindowEventKind::TextCommitted`] when editing ends, instead of parsing them.
pub struct NumberField {
    view: ViewActor,
    field: crate::actor::ActorRef,
    stepper: crate::actor::ActorRef,
    formatter: crate::actor::ActorRef,
    /// The caller-formatted text, or `None` while the number formatter parses input.
    text: std::rc::Rc<std::cell::RefCell<Option<String>>>,
    /// Whether new text waits for editing to end instead of replacing typed text.
    holds_text: std::cell::Cell<bool>,
    accessory: super::field_accessory::AccessorySlot,
}
impl NumberField {
    const STEPPER_WIDTH: f64 = 19.0;
    const SPACING: f64 = 4.0;

    pub fn new(application: &Application) -> Result<Self, UiError> {
        let width = 100.0;
        let height = 22.0;
        // SAFETY: NSView's designated frame initializer returns a retained object.
        let container = unsafe {
            let object = zpd_objc::msg_send!(zpd_objc::class!("NSView"), zpd_objc::sel!("alloc"), () => Id);
            Strong::from_retained(zpd_objc::msg_send!(object, zpd_objc::sel!("initWithFrame:"), ((native::Rect::from(Rect::new(0.0, 0.0, width, height))): native::Rect) => Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        let field = create(zpd_objc::class!("NSTextField"))?;
        let stepper = create(zpd_objc::class!("NSStepper"))?;
        let formatter = create(zpd_objc::class!("NSNumberFormatter"))?;
        let field_frame = Rect::new(
            0.0,
            0.0,
            width - Self::STEPPER_WIDTH - Self::SPACING,
            height,
        );
        let stepper_frame = Rect::new(
            width - Self::STEPPER_WIDTH,
            0.0,
            Self::STEPPER_WIDTH,
            height,
        );
        // SAFETY: All objects are live, retained, and configured on the main thread.
        unsafe {
            zpd_objc::msg_send!(formatter.as_ptr(), zpd_objc::sel!("setNumberStyle:"), ((native::NS_NUMBER_FORMATTER_DECIMAL_STYLE): u64) => ());
            zpd_objc::msg_send!(formatter.as_ptr(), zpd_objc::sel!("setLenient:"), ((true): bool) => ());
            zpd_objc::msg_send!(field.as_ptr(), zpd_objc::sel!("setFormatter:"), ((formatter.as_ptr()): Id) => ());
            zpd_objc::msg_send!(field.as_ptr(), zpd_objc::sel!("setAlignment:"), ((2_i64): native::Integer) => ());
            let cell = zpd_objc::msg_send!(field.as_ptr(), zpd_objc::sel!("cell"), () => Id);
            zpd_objc::msg_send!(cell, zpd_objc::sel!("setSendsActionOnEndEditing:"), ((true): bool) => ());
            zpd_objc::msg_send!(field.as_ptr(), zpd_objc::sel!("setFrame:"), ((native::Rect::from(field_frame)): native::Rect) => ());
            zpd_objc::msg_send!(field.as_ptr(), zpd_objc::sel!("setAutoresizingMask:"), ((native::NS_VIEW_WIDTH_SIZABLE | native::NS_VIEW_HEIGHT_SIZABLE): u64) => ());
            zpd_objc::msg_send!(stepper.as_ptr(), zpd_objc::sel!("setValueWraps:"), ((false): bool) => ());
            zpd_objc::msg_send!(stepper.as_ptr(), zpd_objc::sel!("setAutorepeat:"), ((true): bool) => ());
            zpd_objc::msg_send!(stepper.as_ptr(), zpd_objc::sel!("setFrame:"), ((native::Rect::from(stepper_frame)): native::Rect) => ());
            zpd_objc::msg_send!(stepper.as_ptr(), zpd_objc::sel!("setAutoresizingMask:"), ((native::NS_VIEW_MIN_X_MARGIN | native::NS_VIEW_HEIGHT_SIZABLE): u64) => ());
            zpd_objc::msg_send!(container.as_ptr(), zpd_objc::sel!("setAutoresizesSubviews:"), ((true): bool) => ());
            zpd_objc::msg_send!(container.as_ptr(), zpd_objc::sel!("addSubview:"), ((field.as_ptr()): Id) => ());
            zpd_objc::msg_send!(container.as_ptr(), zpd_objc::sel!("addSubview:"), ((stepper.as_ptr()): Id) => ());
        }
        let view = ViewActor::new(application, container);
        let tree = view.actor().tree_handle().ok_or(UiError::Closed)?;
        let field_actor = tree.insert_child(view.actor(), field)?;
        let stepper_actor = tree.insert_child(view.actor(), stepper)?;
        let formatter_actor = tree.insert_child(&field_actor, formatter)?;
        let text = std::rc::Rc::new(std::cell::RefCell::new(None::<String>));

        let emit = std::rc::Rc::new(callback::emitter(view.actor()));
        let field_emit = emit.clone();
        let stepper_for_field = stepper_actor.clone();
        let field_text = text.clone();
        callback::install_action(&field_actor, move |field| {
            if let Some(shown) = field_text.borrow().as_deref() {
                // SAFETY: The sender is the live field without a formatter.
                let value = unsafe { string_value(field) };
                // The caller parses the text; until it re-renders, the field shows its last text.
                let shown = native::nsstring(shown);
                // SAFETY: The sender is the live field.
                unsafe {
                    zpd_objc::msg_send!(field, zpd_objc::sel!("setStringValue:"), ((shown.as_ptr()): Id) => ())
                };
                field_emit(WindowEventKind::TextCommitted { value });
                return;
            }
            // SAFETY: The sender is the live field; its formatter parsed `doubleValue`.
            let value =
                unsafe { zpd_objc::msg_send!(field, zpd_objc::sel!("doubleValue"), () => f64) };
            let _ = stepper_for_field.with(|stepper| unsafe {
                zpd_objc::msg_send!(stepper, zpd_objc::sel!("setDoubleValue:"), ((value): f64) => ())
            });
            field_emit(WindowEventKind::ValueChanged { value });
        })?;
        // Caller-formatted fields report typing, so the caller sees values before editing ends.
        let typing_emit = emit.clone();
        let typing_text = text.clone();
        callback::install_delegate(&field_actor, move |notification| {
            if typing_text.borrow().is_none() {
                return;
            }
            // SAFETY: Text notifications carry the live field as their object.
            let value = unsafe {
                string_value(zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id))
            };
            typing_emit(WindowEventKind::TextChanged { value });
        })?;
        let field_for_stepper = field_actor.clone();
        let stepper_text = text.clone();
        callback::install_action(&stepper_actor, move |stepper| {
            // SAFETY: The sender is the live stepper; the field is updated to match.
            let value =
                unsafe { zpd_objc::msg_send!(stepper, zpd_objc::sel!("doubleValue"), () => f64) };
            // Caller-formatted text follows the value when the caller re-renders.
            if stepper_text.borrow().is_none() {
                let _ = field_for_stepper.with(|field| unsafe {
                    zpd_objc::msg_send!(field, zpd_objc::sel!("setDoubleValue:"), ((value): f64) => ())
                });
            }
            emit(WindowEventKind::ValueChanged { value });
        })?;
        Ok(Self {
            view,
            field: field_actor,
            stepper: stepper_actor,
            formatter: formatter_actor,
            text,
            holds_text: std::cell::Cell::new(false),
            accessory: Default::default(),
        })
    }

    /// While `hold` is set, [`NumberField::set_text`] keeps what the user is
    /// typing and shows the new text once editing ends.
    ///
    /// Set it while the caller renders the value parsed from typed text, so
    /// the field is not reformatted under the caret.
    pub fn hold_text(&self, hold: bool) {
        self.holds_text.set(hold);
    }

    /// Shows `text` instead of the formatted value, or `None` to format it again.
    pub fn set_text(&self, text: Option<&str>) -> Result<(), UiError> {
        let was_custom = self.text.replace(text.map(str::to_owned)).is_some();
        let formatter = if text.is_some() {
            zpd_objc::NIL
        } else {
            self.formatter.with(|formatter| formatter)?
        };
        let holds = self.holds_text.get();
        self.field.with(|field| {
            // SAFETY: The field is a live NSTextField; the formatter is its live child or nil.
            unsafe {
                if text.is_some() != was_custom {
                    zpd_objc::msg_send!(field, zpd_objc::sel!("setFormatter:"), ((formatter): Id) => ());
                }
                // A field being edited shows held text when editing ends, from `text`.
                let editing = !zpd_objc::msg_send!(field, zpd_objc::sel!("currentEditor"), () => Id)
                    .is_null();
                if let Some(text) = text.filter(|_| !(holds && editing)) {
                    let text = native::nsstring(text);
                    zpd_objc::msg_send!(field, zpd_objc::sel!("setStringValue:"), ((text.as_ptr()): Id) => ());
                }
            }
        })?;
        Ok(())
    }
    /// Shows `placeholder` while the field is empty; `None` removes it.
    pub fn set_placeholder(&self, placeholder: Option<&str>) -> Result<(), UiError> {
        let placeholder = placeholder.map(native::nsstring);
        self.field.with(|field| {
            // SAFETY: The Actor holds a live NSTextField, which copies the string.
            unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setPlaceholderString:"), ((placeholder.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): Id) => ())
            }
        })?;
        Ok(())
    }
    pub fn value(&self) -> Result<f64, UiError> {
        self.stepper
            .with(|stepper| {
                // SAFETY: The Actor holds a live NSStepper.
                unsafe { zpd_objc::msg_send!(stepper, zpd_objc::sel!("doubleValue"), () => f64) }
            })
            .map_err(Into::into)
    }
    pub fn set_value(&self, value: f64) -> Result<(), UiError> {
        let custom = self.text.borrow().is_some();
        let controls = if custom {
            vec![&self.stepper]
        } else {
            vec![&self.field, &self.stepper]
        };
        for control in controls {
            control.with(|control| {
                // SAFETY: Both Actors hold live NSControls accepting doubles.
                unsafe {
                    zpd_objc::msg_send!(control, zpd_objc::sel!("setDoubleValue:"), ((value): f64) => ())
                }
            })?;
        }
        Ok(())
    }
    pub fn set_format(&self, format: NumberFormat) -> Result<(), UiError> {
        self.stepper.with(|stepper| {
            // SAFETY: The Actor holds a live NSStepper.
            unsafe {
                zpd_objc::msg_send!(stepper, zpd_objc::sel!("setMinValue:"), ((format.minimum): f64) => ());
                zpd_objc::msg_send!(stepper, zpd_objc::sel!("setMaxValue:"), ((format.maximum): f64) => ());
                zpd_objc::msg_send!(stepper, zpd_objc::sel!("setIncrement:"), ((format.step): f64) => ());
            }
        })?;
        self.field.with(|field| {
            // SAFETY: The field owns a live NSNumberFormatter configured in `new`.
            unsafe {
                let formatter = zpd_objc::msg_send!(field, zpd_objc::sel!("formatter"), () => Id);
                let number = |value: f64| zpd_objc::msg_send!(zpd_objc::class!("NSNumber"), zpd_objc::sel!("numberWithDouble:"), ((value): f64) => Id);
                zpd_objc::msg_send!(formatter, zpd_objc::sel!("setMinimum:"), ((number(format.minimum)): Id) => ());
                zpd_objc::msg_send!(formatter, zpd_objc::sel!("setMaximum:"), ((number(format.maximum)): Id) => ());
                zpd_objc::msg_send!(formatter, zpd_objc::sel!("setAllowsFloats:"), ((format.fraction_digits > 0): bool) => ());
                zpd_objc::msg_send!(formatter, zpd_objc::sel!("setMinimumFractionDigits:"), ((format.fraction_digits as u64): u64) => ());
                zpd_objc::msg_send!(formatter, zpd_objc::sel!("setMaximumFractionDigits:"), ((format.fraction_digits as u64): u64) => ());
            }
        })?;
        Ok(())
    }
    /// Allows or prevents typing; a read-only field hides its stepper and widens its text field.
    ///
    /// Its text stays selectable, and it is still a text field to accessibility.
    pub fn set_editable(&self, editable: bool) -> Result<(), UiError> {
        let width = self.view.as_view().with(|container| {
            // SAFETY: The live container reports its bounds.
            unsafe { zpd_objc::msg_send!(container, zpd_objc::sel!("bounds"), () => native::Rect) }
                .size
                .width
        })?;
        let stepper = if editable {
            Self::STEPPER_WIDTH + Self::SPACING
        } else {
            0.0
        };
        self.field.with(|field| {
            // SAFETY: The live field and its cell accept these setters; the cell copies the role.
            unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setEditable:"), ((editable): bool) => ());
                zpd_objc::msg_send!(field, zpd_objc::sel!("setSelectable:"), ((true): bool) => ());
                // AppKit reports non-editable fields as static text.
                let cell = zpd_objc::msg_send!(field, zpd_objc::sel!("cell"), () => Id);
                zpd_objc::msg_send!(cell, zpd_objc::sel!("setAccessibilityRole:"), ((native::nsstring("AXTextField").as_ptr()): Id) => ());
                let mut frame = zpd_objc::msg_send!(field, zpd_objc::sel!("frame"), () => native::Rect);
                frame.size.width = (width - stepper).max(0.0);
                zpd_objc::msg_send!(field, zpd_objc::sel!("setFrame:"), ((frame): native::Rect) => ());
            }
        })?;
        ViewRef::from_actor(&self.stepper).set_hidden(!editable)
    }
    /// Returns the text field, which VoiceOver reads as the number.
    pub fn accessibility_view(&self) -> ViewRef<'_> {
        ViewRef::from_actor(&self.field)
    }
    /// Shows `accessory` inside the text field's trailing edge, before the stepper, or removes it for `None`.
    ///
    /// Clicks are reported as [`WindowEventKind::AccessoryClicked`].
    pub fn set_accessory(&self, accessory: Option<&super::FieldAccessory>) -> Result<(), UiError> {
        self.accessory
            .set(&self.field, self.view.actor(), accessory)
    }
    /// Makes the text field the first responder of its window, if it is in one.
    pub fn focus(&self) -> Result<(), UiError> {
        self.field.with(|field| {
            // SAFETY: The live field and its window, which may be nil, accept the message.
            unsafe {
                let window = zpd_objc::msg_send!(field, zpd_objc::sel!("window"), () => Id);
                if !window.is_null() {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("makeFirstResponder:"), ((field): Id) => bool);
                }
            }
        })?;
        Ok(())
    }
    /// Returns the height of the taller of the text field and stepper, with no fixed width.
    ///
    /// The container is a plain `NSView`, which has no intrinsic size of its own.
    pub fn intrinsic_content_size(&self) -> Result<(Option<f64>, Option<f64>), UiError> {
        let (_, field) = ViewRef::from_actor(&self.field).intrinsic_content_size()?;
        let (_, stepper) = ViewRef::from_actor(&self.stepper).intrinsic_content_size()?;
        let height = match (field, stepper) {
            (Some(field), Some(stepper)) => Some(field.max(stepper)),
            (field, stepper) => field.or(stepper),
        };
        Ok((None, height))
    }
    /// Shows `text` over both the field and its stepper.
    pub fn set_tool_tip(&self, text: Option<&str>) -> Result<(), UiError> {
        for control in [&self.field, &self.stepper] {
            ViewRef::from_actor(control).set_tool_tip(text)?;
        }
        Ok(())
    }
    /// Enables or disables both the field and its stepper.
    pub fn set_enabled(&self, enabled: bool) -> Result<(), UiError> {
        for control in [&self.field, &self.stepper] {
            ViewRef::from_actor(control).set_enabled(enabled)?;
        }
        Ok(())
    }
}

/// A native `NSSearchField` that reports every edit.
pub struct SearchField {
    view: ViewActor,
}
impl SearchField {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        let view = ViewActor::new(application, create(zpd_objc::class!("NSSearchField"))?);
        let emit = std::rc::Rc::new(callback::emitter(view.actor()));
        let text_emit = emit.clone();
        callback::install_delegate(view.actor(), move |notification| {
            // SAFETY: Text notifications carry the live NSSearchField as their object.
            let value = unsafe {
                string_value(zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id))
            };
            text_emit(WindowEventKind::TextChanged { value });
        })?;
        // The action also fires when the cancel button clears the field.
        callback::install_action(view.actor(), move |field| {
            // SAFETY: The action sender is the live NSSearchField.
            let value = unsafe { string_value(field) };
            emit(WindowEventKind::TextChanged { value });
        })?;
        Ok(Self { view })
    }
    pub fn string_value(&self) -> Result<String, UiError> {
        // SAFETY: The live view is an NSSearchField.
        self.view
            .as_view()
            .with(|field| unsafe { string_value(field) })
    }
    pub fn set_string_value(&self, value: &str) -> Result<(), UiError> {
        let value = native::nsstring(value);
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSSearchField.
            unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setStringValue:"), ((value.as_ptr()): Id) => ())
            }
        })
    }
    pub fn set_placeholder(&self, value: Option<&str>) -> Result<(), UiError> {
        let value = value.map(native::nsstring);
        self.view.as_view().with(|field| {
            // SAFETY: The live view is an NSSearchField; nil restores the system placeholder.
            unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setPlaceholderString:"), ((value.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): Id) => ())
            }
        })
    }
}

/// A native `NSColorWell` that opens the shared color panel.
pub struct ColorWell {
    view: ViewActor,
}
impl ColorWell {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        let view = ViewActor::new(application, create(zpd_objc::class!("NSColorWell"))?);
        let emit = callback::emitter(view.actor());
        callback::install_action(view.actor(), move |well| {
            // SAFETY: The action sender is the live NSColorWell.
            let color = unsafe {
                Color::from_native(zpd_objc::msg_send!(well, zpd_objc::sel!("color"), () => Id))
            };
            emit(WindowEventKind::ColorChanged { color });
        })?;
        Ok(Self { view })
    }
    pub fn color(&self) -> Result<Color, UiError> {
        self.view.as_view().with(|well| {
            // SAFETY: The live view is an NSColorWell.
            unsafe {
                Color::from_native(zpd_objc::msg_send!(well, zpd_objc::sel!("color"), () => Id))
            }
        })
    }
    pub fn set_color(&self, color: Color) -> Result<(), UiError> {
        self.view.as_view().with(|well| {
            // SAFETY: The live view is an NSColorWell; the color is autoreleased.
            unsafe {
                zpd_objc::msg_send!(well, zpd_objc::sel!("setColor:"), ((color.native()): Id) => ())
            }
        })
    }
}
