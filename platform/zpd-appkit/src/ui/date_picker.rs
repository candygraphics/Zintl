use crate::actor::WindowEventKind;
use crate::native;
use crate::runloop::Application;
use zpd_objc::Id;

use super::UiError;
use super::callback;
use super::view::{AsView, ViewActor, ViewRef};

/// How a [`DatePicker`] shows its date.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerStyle {
    /// A text field with a stepper.
    #[default]
    TextField,
    /// A month calendar.
    Graphical,
}

impl DatePickerStyle {
    /// Returns the `NSDatePickerStyle` value.
    fn native(self) -> native::Integer {
        match self {
            Self::TextField => 0,
            Self::Graphical => 1,
        }
    }
}

/// An `NSDatePicker` that edits a calendar date without time or time zone.
///
/// The picker and its conversions use a Gregorian calendar in UTC, so a date
/// stays the same day in every local time zone. Formatting follows the
/// system locale.
pub struct DatePicker {
    view: ViewActor,
}

impl DatePicker {
    /// `NSDatePickerElementFlagYearMonthDay`.
    const YEAR_MONTH_DAY: u64 = 0x00e0;
    /// The calendar units `NSCalendarUnitYear | NSCalendarUnitMonth | NSCalendarUnitDay`.
    const DATE_UNITS: u64 = (1 << 2) | (1 << 3) | (1 << 4);

    pub fn new(application: &Application) -> Result<Self, UiError> {
        // SAFETY: NSDatePicker alloc/init returns a retained view; the calendar and time zone
        // are autoreleased objects the picker retains.
        let picker = unsafe {
            let object = zpd_objc::msg_send!(zpd_objc::class!("NSDatePicker"), zpd_objc::sel!("alloc"), () => Id);
            let picker = zpd_objc::Strong::from_retained(
                zpd_objc::msg_send!(object, zpd_objc::sel!("init"), () => Id),
            )
            .ok_or(UiError::NativeCreationFailed)?;
            let calendar = Self::calendar();
            zpd_objc::msg_send!(picker.as_ptr(), zpd_objc::sel!("setCalendar:"), ((calendar): Id) => ());
            let utc = zpd_objc::msg_send!(calendar, zpd_objc::sel!("timeZone"), () => Id);
            zpd_objc::msg_send!(picker.as_ptr(), zpd_objc::sel!("setTimeZone:"), ((utc): Id) => ());
            zpd_objc::msg_send!(picker.as_ptr(), zpd_objc::sel!("setDatePickerElements:"), ((Self::YEAR_MONTH_DAY): u64) => ());
            zpd_objc::msg_send!(picker.as_ptr(), zpd_objc::sel!("setDrawsBackground:"), ((true): bool) => ());
            picker
        };
        let view = ViewActor::new(application, picker);
        let emit = callback::emitter(view.actor());
        callback::install_action(view.actor(), move |picker| {
            // SAFETY: The action sender is the live NSDatePicker.
            let (year, month, day) = unsafe { Self::components(picker) };
            emit(WindowEventKind::DateChanged { year, month, day });
        })?;
        Ok(Self { view })
    }

    /// Returns an autoreleased Gregorian calendar in UTC.
    ///
    /// # Safety
    /// Must run on the main thread inside an autorelease scope.
    unsafe fn calendar() -> Id {
        // SAFETY: Guaranteed by the caller; the calendar copies its time zone.
        unsafe {
            let identifier = native::nsstring("gregorian");
            let calendar = zpd_objc::msg_send!(zpd_objc::class!("NSCalendar"), zpd_objc::sel!("calendarWithIdentifier:"), ((identifier.as_ptr()): Id) => Id);
            let utc = zpd_objc::msg_send!(zpd_objc::class!("NSTimeZone"), zpd_objc::sel!("timeZoneForSecondsFromGMT:"), ((0): native::Integer) => Id);
            zpd_objc::msg_send!(calendar, zpd_objc::sel!("setTimeZone:"), ((utc): Id) => ());
            calendar
        }
    }

    /// Reads the picker's date as `(year, month, day)`.
    ///
    /// # Safety
    /// `picker` must be a live NSDatePicker on the main thread.
    unsafe fn components(picker: Id) -> (i32, u8, u8) {
        // SAFETY: Guaranteed by the caller; the calendar and components are autoreleased.
        unsafe {
            let date = zpd_objc::msg_send!(picker, zpd_objc::sel!("dateValue"), () => Id);
            let components = zpd_objc::msg_send!(Self::calendar(), zpd_objc::sel!("components:fromDate:"), ((Self::DATE_UNITS): u64, (date): Id) => Id);
            let year =
                zpd_objc::msg_send!(components, zpd_objc::sel!("year"), () => native::Integer);
            let month =
                zpd_objc::msg_send!(components, zpd_objc::sel!("month"), () => native::Integer);
            let day = zpd_objc::msg_send!(components, zpd_objc::sel!("day"), () => native::Integer);
            (year as i32, month as u8, day as u8)
        }
    }

    /// Returns the shown date as `(year, month, day)`.
    pub fn date(&self) -> Result<(i32, u8, u8), UiError> {
        // SAFETY: The live view is an NSDatePicker.
        self.view
            .as_view()
            .with(|picker| unsafe { Self::components(picker) })
    }

    /// Shows the date `year`-`month`-`day` without reporting it.
    pub fn set_date(&self, year: i32, month: u8, day: u8) -> Result<(), UiError> {
        self.view.as_view().with(|picker| {
            // SAFETY: The live view is an NSDatePicker; the components and date are autoreleased.
            unsafe {
                let components = native::alloc_init(zpd_objc::class!("NSDateComponents"));
                zpd_objc::msg_send!(components.as_ptr(), zpd_objc::sel!("setYear:"), ((year as native::Integer): native::Integer) => ());
                zpd_objc::msg_send!(components.as_ptr(), zpd_objc::sel!("setMonth:"), ((month as native::Integer): native::Integer) => ());
                zpd_objc::msg_send!(components.as_ptr(), zpd_objc::sel!("setDay:"), ((day as native::Integer): native::Integer) => ());
                let date = zpd_objc::msg_send!(Self::calendar(), zpd_objc::sel!("dateFromComponents:"), ((components.as_ptr()): Id) => Id);
                if !date.is_null() {
                    zpd_objc::msg_send!(picker, zpd_objc::sel!("setDateValue:"), ((date): Id) => ());
                }
            }
        })
    }

    pub fn set_style(&self, style: DatePickerStyle) -> Result<(), UiError> {
        self.view.as_view().with(|picker| {
            // SAFETY: The live view is an NSDatePicker and the value is an NSDatePickerStyle.
            unsafe {
                zpd_objc::msg_send!(picker, zpd_objc::sel!("setDatePickerStyle:"), ((style.native()): native::Integer) => ());
                // The calendar style draws no bezel.
                zpd_objc::msg_send!(picker, zpd_objc::sel!("setBezeled:"), ((style == DatePickerStyle::TextField): bool) => ());
            }
        })
    }
}

impl AsView for DatePicker {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}
