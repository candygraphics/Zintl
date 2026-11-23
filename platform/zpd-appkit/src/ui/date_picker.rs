use std::sync::OnceLock;

use crate::actor::WindowEventKind;
use crate::native;
use crate::runloop::Application;
use zpd_objc::Id;

use super::UiError;

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    fn NSBeep();
}
use super::callback;
use super::pasteboard::Pasteboard;
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
/// system locale. Accessibility reports the date in ISO 8601 form, such as
/// `2026-11-15`, and pasting an ISO date sets it.
pub struct DatePicker {
    view: ViewActor,
}

/// Returns the date `picker` shows as an autoreleased ISO 8601 string.
unsafe extern "C" fn accessibility_value(picker: Id, _: zpd_objc::Sel) -> Id {
    zpd_objc::abort_on_unwind(|| {
        // SAFETY: AppKit asks the live picker; `stringWithString:` returns an autoreleased copy.
        unsafe {
            let (year, month, day) = DatePicker::components(picker);
            let text = native::nsstring(&DatePicker::iso(year, month, day));
            zpd_objc::msg_send!(zpd_objc::class!("NSString"), zpd_objc::sel!("stringWithString:"), ((text.as_ptr()): Id) => Id)
        }
    })
}

/// Returns the localized date VoiceOver speaks for the ISO value.
unsafe extern "C" fn accessibility_value_description(picker: Id, _: zpd_objc::Sel) -> Id {
    zpd_objc::abort_on_unwind(|| {
        // SAFETY: AppKit asks the live picker; the formatter and its string are autoreleased.
        unsafe {
            let formatter = zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSDateFormatter"), zpd_objc::sel!("new"), () => Id), zpd_objc::sel!("autorelease"), () => Id);
            // NSDateFormatterLongStyle, without time.
            zpd_objc::msg_send!(formatter, zpd_objc::sel!("setDateStyle:"), ((3): u64) => ());
            zpd_objc::msg_send!(formatter, zpd_objc::sel!("setTimeStyle:"), ((0): u64) => ());
            let calendar = DatePicker::calendar();
            zpd_objc::msg_send!(formatter, zpd_objc::sel!("setCalendar:"), ((calendar): Id) => ());
            let utc = zpd_objc::msg_send!(calendar, zpd_objc::sel!("timeZone"), () => Id);
            zpd_objc::msg_send!(formatter, zpd_objc::sel!("setTimeZone:"), ((utc): Id) => ());
            let date = zpd_objc::msg_send!(picker, zpd_objc::sel!("dateValue"), () => Id);
            zpd_objc::msg_send!(formatter, zpd_objc::sel!("stringFromDate:"), ((date): Id) => Id)
        }
    })
}

/// Sets the picker to an ISO 8601 date on the pasteboard and reports it like a user change.
unsafe extern "C" fn paste(picker: Id, _: zpd_objc::Sel, _: Id) {
    zpd_objc::abort_on_unwind(|| {
        // SAFETY: AppKit sends `paste:` to the live picker on the main thread.
        unsafe {
            let parsed = Pasteboard::string().and_then(|text| DatePicker::parse_iso(&text));
            let Some((year, month, day)) = parsed else {
                NSBeep();
                return;
            };
            DatePicker::show(picker, year, month, day);
            let action = zpd_objc::msg_send!(picker, zpd_objc::sel!("action"), () => zpd_objc::Sel);
            let target = zpd_objc::msg_send!(picker, zpd_objc::sel!("target"), () => Id);
            zpd_objc::msg_send!(picker, zpd_objc::sel!("sendAction:to:"), ((action): zpd_objc::Sel, (target): Id) => bool);
        }
    })
}

impl DatePicker {
    /// `NSDatePickerElementFlagYearMonthDay`.
    const YEAR_MONTH_DAY: u64 = 0x00e0;
    /// The calendar units `NSCalendarUnitYear | NSCalendarUnitMonth | NSCalendarUnitDay`.
    const DATE_UNITS: u64 = (1 << 2) | (1 << 3) | (1 << 4);

    fn class() -> zpd_objc::Class {
        static CLASS: OnceLock<usize> = OnceLock::new();
        *CLASS.get_or_init(|| {
            zpd_objc::decl!(ZpdDatePicker: [zpd_objc::class!("NSDatePicker")] {
                fields {}
                methods {
                    "accessibilityValue": "@@:" => accessibility_value,
                    "accessibilityValueDescription": "@@:" => accessibility_value_description,
                    "paste:": "v@:@" => paste,
                }
            }) as usize
        }) as zpd_objc::Class
    }

    /// Formats a date as ISO 8601, such as `2026-11-15`.
    fn iso(year: i32, month: u8, day: u8) -> String {
        format!("{year:04}-{month:02}-{day:02}")
    }

    /// Parses an ISO 8601 date such as `2026-11-15`, rejecting days that do not exist.
    pub fn parse_iso(text: &str) -> Option<(i32, u8, u8)> {
        let mut parts = text.trim().splitn(3, '-');
        let year = parts.next()?.parse::<i32>().ok()?;
        let month = parts.next()?.parse::<u8>().ok()?;
        let day = parts.next()?.parse::<u8>().ok()?;
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => return None,
        };
        (1..=days).contains(&day).then_some((year, month, day))
    }

    pub fn new(application: &Application) -> Result<Self, UiError> {
        // SAFETY: NSDatePicker alloc/init returns a retained view; the calendar and time zone
        // are autoreleased objects the picker retains.
        let picker = unsafe {
            let object = zpd_objc::msg_send!(Self::class(), zpd_objc::sel!("alloc"), () => Id);
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

    /// Shows the date `year`-`month`-`day` in `picker`.
    ///
    /// # Safety
    /// `picker` must be a live NSDatePicker on the main thread.
    unsafe fn show(picker: Id, year: i32, month: u8, day: u8) {
        // SAFETY: Guaranteed by the caller; the components and date are autoreleased.
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
    }

    /// Shows the date `year`-`month`-`day` without reporting it.
    pub fn set_date(&self, year: i32, month: u8, day: u8) -> Result<(), UiError> {
        // SAFETY: The live view is an NSDatePicker.
        self.view
            .as_view()
            .with(|picker| unsafe { Self::show(picker, year, month, day) })
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

#[cfg(test)]
mod tests {
    use super::DatePicker;

    #[test]
    fn parses_iso_dates_that_exist() {
        // Verifies pasted ISO dates are read, including leap days, and impossible days are rejected.
        assert_eq!(DatePicker::parse_iso("2024-02-29"), Some((2024, 2, 29)));
        assert_eq!(DatePicker::parse_iso(" 2026-11-15\n"), Some((2026, 11, 15)));
        assert_eq!(DatePicker::parse_iso("2026-02-29"), None);
        assert_eq!(DatePicker::parse_iso("2026-13-01"), None);
        assert_eq!(DatePicker::parse_iso("15/11/2026"), None);
        assert_eq!(DatePicker::iso(2026, 1, 5), "2026-01-05");
    }
}
