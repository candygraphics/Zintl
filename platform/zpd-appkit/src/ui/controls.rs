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
);

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

pub use crate::model::NumberFormat;

/// A number-only `NSTextField` paired with an `NSStepper`.
pub struct NumberField {
    view: ViewActor,
    field: crate::actor::ActorRef,
    stepper: crate::actor::ActorRef,
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
        tree.insert_child(&field_actor, formatter)?;

        let emit = std::rc::Rc::new(callback::emitter(view.actor()));
        let field_emit = emit.clone();
        let stepper_for_field = stepper_actor.clone();
        callback::install_action(&field_actor, move |field| {
            // SAFETY: The sender is the live field; its formatter parsed `doubleValue`.
            let value =
                unsafe { zpd_objc::msg_send!(field, zpd_objc::sel!("doubleValue"), () => f64) };
            let _ = stepper_for_field.with(|stepper| unsafe {
                zpd_objc::msg_send!(stepper, zpd_objc::sel!("setDoubleValue:"), ((value): f64) => ())
            });
            field_emit(WindowEventKind::ValueChanged { value });
        })?;
        let field_for_stepper = field_actor.clone();
        callback::install_action(&stepper_actor, move |stepper| {
            // SAFETY: The sender is the live stepper; the field is updated to match.
            let value =
                unsafe { zpd_objc::msg_send!(stepper, zpd_objc::sel!("doubleValue"), () => f64) };
            let _ = field_for_stepper.with(|field| unsafe {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setDoubleValue:"), ((value): f64) => ())
            });
            emit(WindowEventKind::ValueChanged { value });
        })?;
        Ok(Self {
            view,
            field: field_actor,
            stepper: stepper_actor,
        })
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
        for control in [&self.field, &self.stepper] {
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
