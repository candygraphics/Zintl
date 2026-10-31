//! Plain UI descriptions that every native backend reads.

use zintl_ui_graphics::{Color, Font};

/// A titled group of rows in a [`crate::ViewKind::List`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListSection {
    pub title: Option<String>,
    /// Row IDs; the list's children are the row views in the same order.
    pub ids: Vec<String>,
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
    Custom(Color),
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

#[derive(Clone, Debug, PartialEq)]
pub struct CommandItem {
    pub id: Option<String>,
    pub title: String,
    pub role: Option<CommandRole>,
    pub key: Option<String>,
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
    /// Buttons from right to left; the first answers Return.
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarSection {
    pub title: Option<String>,
    pub items: Vec<SidebarItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarItem {
    pub id: String,
    pub title: String,
    pub system_image: Option<String>,
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
