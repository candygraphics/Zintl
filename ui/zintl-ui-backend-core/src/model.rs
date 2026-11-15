//! Plain UI descriptions that every native backend reads.

use zintl_ui_graphics::{Color, Font};

/// The items of a window toolbar, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Toolbar {
    pub items: Vec<ToolbarItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolbarItem {
    /// A button with an SF Symbol; clicks report [`crate::Event::ToolbarItemClicked`].
    Button {
        id: String,
        label: String,
        system_image: String,
        enabled: bool,
    },
    /// A search field; edits report [`crate::Event::ToolbarSearchChanged`].
    Search {
        value: String,
        placeholder: Option<String>,
    },
    FlexibleSpace,
}

/// A titled group of rows in a [`crate::ViewKind::List`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListSection {
    pub title: Option<String>,
    /// Row IDs; the list's children are the row views in the same order.
    pub ids: Vec<String>,
}

/// A titled column of a [`crate::ViewKind::Table`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableColumn {
    pub title: String,
    /// The initial rendered width in points; `None` lets the table size it.
    pub width: Option<f64>,
    /// The header's alignment, and the default for text cells that keep the leading alignment.
    pub alignment: TextAlignment,
    /// The key reported when the user sorts by this column; `None` makes it unsortable.
    pub sort_key: Option<String>,
}

/// A calendar date without time or time zone, such as a due date.
///
/// It is not an instant, so it stays the same day in every time zone.
/// Convert it to the date library the app uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    pub year: i32,
    /// From 1 for January.
    pub month: u8,
    /// From 1.
    pub day: u8,
}

impl CalendarDate {
    /// Returns the date, or `None` when the month or day does not exist, such as February 30.
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        (1..=12)
            .contains(&month)
            .then(|| Self::days_in_month(year, month))
            .filter(|days| (1..=*days).contains(&day))
            .map(|_| Self { year, month, day })
    }

    /// Returns the number of days in `month` of `year` in the Gregorian calendar.
    pub fn days_in_month(year: i32, month: u8) -> u8 {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    /// Parses an ISO 8601 date such as `2026-11-15`.
    pub fn parse_iso(text: &str) -> Option<Self> {
        let mut parts = text.trim().splitn(3, '-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        Self::new(year, month, day)
    }
}

impl std::fmt::Display for CalendarDate {
    /// Writes the ISO 8601 form, such as `2026-11-15`.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

/// How a date picker shows its date.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerStyle {
    /// A text field with a stepper.
    #[default]
    TextField,
    /// A month calendar.
    Graphical,
}

/// The order a table's rows should be sorted in; the app sorts them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortOrder {
    /// The sort key of the column the user chose.
    pub key: String,
    pub ascending: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListStyle {
    /// Inset rows with rounded selection, like Notes and Mail.
    #[default]
    Inset,
    /// Rows that span the full width.
    Plain,
}

/// How a text field draws its text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyle {
    /// `None` keeps the control's standard font.
    pub font: Option<Font>,
    pub color: TextColor,
    pub alignment: TextAlignment,
    /// Lines shown before the last one is truncated with an ellipsis.
    ///
    /// `None` keeps the control's own wrapping.
    pub line_limit: Option<usize>,
}

/// A text color that follows the system appearance unless it is `Custom`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextColor {
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Accent,
    /// A platform color that adapts to dark mode and Increase Contrast.
    System(SystemColor),
    Custom(Color),
}

/// A platform color for meaning such as errors or positive amounts.
///
/// It is resolved when the text is drawn, so it follows the appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Gray,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlignment {
    #[default]
    Leading,
    Center,
    Trailing,
}

/// How a button responds to clicks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonType {
    #[default]
    MomentaryPushIn,
    PushOnPushOff,
    Checkbox,
    Radio,
}

/// The shape drawn around a button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BezelStyle {
    #[default]
    Push,
    FlexiblePush,
    Disclosure,
    Circular,
    Help,
    SmallSquare,
    Toolbar,
    AccessoryBarAction,
    AccessoryBar,
    PushDisclosure,
    Badge,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandSet {
    pub app_menu: Option<WindowAppMenu>,
    pub menus: Vec<CommandMenu>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowAppMenu {
    pub items: Vec<CommandItem>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandMenu {
    pub title: String,
    pub items: Vec<CommandItem>,
    /// Marks a menu the platform fills or decorates itself.
    pub role: Option<MenuRole>,
}

/// The key of a menu shortcut: a character, or a key that types none, such as ⌫.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShortcutKey {
    /// The character the key produces without modifiers, such as `"n"`.
    Character(String),
    Named(crate::NamedKey),
}

impl From<&str> for ShortcutKey {
    fn from(character: &str) -> Self {
        Self::Character(character.to_owned())
    }
}

impl From<String> for ShortcutKey {
    fn from(character: String) -> Self {
        Self::Character(character)
    }
}

impl From<crate::NamedKey> for ShortcutKey {
    fn from(key: crate::NamedKey) -> Self {
        Self::Named(key)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommandItem {
    pub id: Option<String>,
    pub title: String,
    pub role: Option<CommandRole>,
    pub key: Option<ShortcutKey>,
    pub modifiers: Vec<CommandModifier>,
    pub enabled: bool,
    pub checked: bool,
    pub submenu: Option<CommandMenu>,
}

impl CommandItem {
    /// Creates an enabled item with no action.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: None,
            title: title.into(),
            role: None,
            key: None,
            modifiers: Vec::new(),
            enabled: true,
            checked: false,
            submenu: None,
        }
    }

    pub fn separator() -> Self {
        Self {
            role: Some(CommandRole::Separator),
            ..Self::new("")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandModifier {
    Cmd,
    Ctrl,
    Alt,
    Shift,
}

/// Standard actions the platform performs itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandRole {
    About,
    Quit,
    Hide,
    HideOthers,
    ShowAll,
    Close,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    Find,
    FindAndReplace,
    FindNext,
    FindPrevious,
    UseSelectionForFind,
    Bold,
    Italic,
    Underline,
    TitleStyle,
    HeadingStyle,
    SubheadingStyle,
    BodyStyle,
    MonospacedStyle,
    BulletedList,
    DashedList,
    NumberedList,
    Checklist,
    ToggleSidebar,
    ToggleFullScreen,
    Minimize,
    Zoom,
    BringAllToFront,
    Help,
    Separator,
}

/// Menus the platform fills or decorates itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuRole {
    Services,
    Windows,
    Help,
}

/// The severity an alert conveys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertStyle {
    #[default]
    Warning,
    Informational,
    Critical,
}

/// How an alert button behaves when the user presses a key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertButtonRole {
    #[default]
    Normal,
    /// Answers the Escape key.
    Cancel,
    /// Draws the button as destructive.
    Destructive,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlertButton {
    pub title: String,
    pub role: AlertButtonRole,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AlertConfig {
    pub style: AlertStyle,
    pub title: String,
    pub message: Option<String>,
    /// Buttons from right to left; the first answers Return unless macOS draws it as destructive.
    pub buttons: Vec<AlertButton>,
}

/// Whether a [`FileDialogConfig`] opens existing items or chooses a save location.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileDialogMode {
    #[default]
    Open,
    Save,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileDialogConfig {
    pub mode: FileDialogMode,
    pub title: Option<String>,
    pub message: Option<String>,
    pub prompt: Option<String>,
    /// Filename extensions without dots; empty allows every type.
    pub allowed_extensions: Vec<String>,
    pub can_choose_files: bool,
    pub can_choose_directories: bool,
    pub allows_multiple: bool,
    pub can_create_directories: bool,
    pub file_name: Option<String>,
    pub directory: Option<String>,
}

/// Numeric bounds and formatting for a number field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberFormat {
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
    pub fraction_digits: usize,
}

/// The side of the anchor view where a popover prefers to appear.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopoverEdge {
    Leading,
    Trailing,
    Top,
    #[default]
    Bottom,
}

/// Where a revealed child sits in a scroll view's visible area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAlignment {
    /// Scrolls only as far as needed to show the whole child.
    #[default]
    Nearest,
    Start,
    Center,
    End,
}

/// The child a scroll view keeps in view, by its view ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrollTarget {
    pub id: String,
    pub alignment: ScrollAlignment,
    /// Changes on every request, so the same ID can be revealed again.
    pub revision: u64,
}

/// Directions in which a scroll view scrolls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollAxes {
    pub horizontal: bool,
    pub vertical: bool,
}

impl ScrollAxes {
    pub const VERTICAL: Self = Self {
        horizontal: false,
        vertical: true,
    };
    pub const HORIZONTAL: Self = Self {
        horizontal: true,
        vertical: false,
    };
    pub const BOTH: Self = Self {
        horizontal: true,
        vertical: true,
    };
}

impl Default for ScrollAxes {
    fn default() -> Self {
        Self::VERTICAL
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sidebar {
    pub sections: Vec<SidebarSection>,
    pub selected_id: Option<String>,
    /// Shows a search field above the items.
    pub search: Option<SidebarSearch>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SidebarSearch {
    pub value: String,
    pub placeholder: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SidebarSection {
    pub title: Option<String>,
    pub items: Vec<SidebarItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SidebarItem {
    pub id: String,
    pub title: String,
    pub system_image: Option<String>,
    /// Trailing text such as a count.
    pub badge: Option<String>,
    /// The item's context menu; selections report `Event::MenuItemSelected` on the window.
    pub menu: Vec<CommandItem>,
}

/// How the title bar relates to the window content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TitleBar {
    #[default]
    Standard,
    /// Content extends under the visible title bar.
    Overlay,
    /// Content fills the window; only the window buttons stay above it.
    ///
    /// On macOS the title and title bar background are hidden. On Windows the
    /// content extends into the title bar, like `Overlay`.
    Hidden,
}

/// Selects the native window class and chrome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowStyle {
    /// A standard titled, resizable window.
    #[default]
    Normal,
    /// A floating utility panel that hides while the app is inactive.
    Panel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_dates_validate_month_lengths_and_leap_years() {
        // Verifies month ends, February 29 in leap years only, and ISO round trips.
        assert!(CalendarDate::new(2024, 2, 29).is_some());
        assert!(CalendarDate::new(2026, 2, 29).is_none());
        assert!(CalendarDate::new(1900, 2, 29).is_none());
        assert!(CalendarDate::new(2000, 2, 29).is_some());
        assert!(CalendarDate::new(2026, 4, 31).is_none());
        assert!(CalendarDate::new(2026, 13, 1).is_none());
        let end = CalendarDate::new(2026, 12, 31).unwrap();
        assert_eq!(end.to_string(), "2026-12-31");
        assert_eq!(
            CalendarDate::parse_iso("2027-01-01"),
            CalendarDate::new(2027, 1, 1)
        );
        assert_eq!(CalendarDate::parse_iso("2026-02-30"), None);
    }
}
