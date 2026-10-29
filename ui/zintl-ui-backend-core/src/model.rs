//! Plain UI descriptions that every native backend reads.

/// Rectangle in logical points / DIP.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Semantic AppKit events supported by the Actor tree.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowEventKind {
    Created,
    DidResize,
    WillClose,
    DidClose,
    ButtonClicked,
    TextChanged {
        value: String,
    },
    SidebarSelectionChanged {
        id: String,
    },
    SidebarSearchChanged {
        value: String,
    },
    /// A two-state control such as a switch, checkbox, or toggle button changed.
    StateChanged {
        on: bool,
    },
    /// A continuous control such as a slider or stepper changed.
    ValueChanged {
        value: f64,
    },
    /// A choice control such as a pop-up button, segmented control, or tab view changed.
    SelectionChanged {
        index: usize,
    },
    ColorChanged {
        color: Color,
    },
    /// A scroll view's top-left document point moved.
    Scrolled {
        x: f64,
        y: f64,
    },
    MenuItemSelected {
        id: String,
    },
    PopoverClosed,
    /// An alert ended; `button` is `None` when it was dismissed programmatically.
    AlertResponse {
        button: Option<usize>,
    },
    /// A file panel ended; `paths` is `None` when it was cancelled.
    FileDialogCompleted {
        paths: Option<Vec<String>>,
    },
    ColorPanelClosed,
}

/// An sRGB color with components in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Color {
    pub const fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

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
    /// Registers the menu with `NSApp` so AppKit can manage its contents.
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

/// Standard actions delivered through the AppKit responder chain.
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
    ToggleSidebar,
    ToggleFullScreen,
    Minimize,
    Zoom,
    BringAllToFront,
    Help,
    Separator,
}

/// Menus that `NSApp` fills or decorates itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuRole {
    Services,
    Windows,
    Help,
}

/// Mirrors `NSAlertStyle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u64)]
pub enum AlertStyle {
    #[default]
    Warning = 0,
    Informational = 1,
    Critical = 2,
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

/// Scroll directions enabled on a scroll view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollAxes {
    pub horizontal: bool,
    pub vertical: bool,
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

/// Selects the native window class and chrome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowStyle {
    /// A standard titled, resizable window.
    #[default]
    Normal,
    /// A floating utility panel that hides while the app is inactive.
    Panel,
}
