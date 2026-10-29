//! Plain AppKit configuration values shared by render nodes and views.

/// An sRGB color with components in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    pub const fn rgb(red: f64, green: f64, blue: f64) -> Self {
        Self::rgba(red, green, blue, 1.0)
    }

    pub const fn rgba(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

/// Mirrors the `NSButtonType` behaviors exposed by [`crate::Button`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonType {
    #[default]
    MomentaryPushIn,
    /// Stays pressed until clicked again.
    PushOnPushOff,
    Checkbox,
    Radio,
}

/// Mirrors `NSBezelStyle`.
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

/// The semantic role of a button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonRole {
    #[default]
    Normal,
    /// Answers Return and draws with the accent color.
    Default,
    /// Answers Escape.
    Cancel,
    /// Marks an action that destroys data.
    Destructive,
}

/// Selects a standard window or a floating utility panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowStyle {
    #[default]
    Normal,
    Panel,
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

/// Mirrors `NSAlertStyle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertStyle {
    #[default]
    Warning,
    Informational,
    Critical,
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

/// The saved state of a scroll view, bound with [`crate::ScrollView::bind`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollStatus {
    /// The horizontal document offset shown at the leading edge.
    pub x: f64,
    /// The vertical document offset shown at the top edge.
    pub y: f64,
}

mod backend {
    use super::*;
    use zintl_ui_backend_core as native;

    impl Color {
        pub(crate) fn backend(self) -> native::Color {
            native::Color::new(self.red, self.green, self.blue, self.alpha)
        }

        pub(crate) fn from_backend(color: native::Color) -> Self {
            Self::rgba(color.red, color.green, color.blue, color.alpha)
        }
    }

    impl ButtonType {
        pub(crate) fn backend(self) -> native::ButtonType {
            match self {
                Self::MomentaryPushIn => native::ButtonType::MomentaryPushIn,
                Self::PushOnPushOff => native::ButtonType::PushOnPushOff,
                Self::Checkbox => native::ButtonType::Checkbox,
                Self::Radio => native::ButtonType::Radio,
            }
        }
    }

    impl BezelStyle {
        pub(crate) fn backend(self) -> native::BezelStyle {
            match self {
                Self::Push => native::BezelStyle::Push,
                Self::FlexiblePush => native::BezelStyle::FlexiblePush,
                Self::Disclosure => native::BezelStyle::Disclosure,
                Self::Circular => native::BezelStyle::Circular,
                Self::Help => native::BezelStyle::Help,
                Self::SmallSquare => native::BezelStyle::SmallSquare,
                Self::Toolbar => native::BezelStyle::Toolbar,
                Self::AccessoryBarAction => native::BezelStyle::AccessoryBarAction,
                Self::AccessoryBar => native::BezelStyle::AccessoryBar,
                Self::PushDisclosure => native::BezelStyle::PushDisclosure,
                Self::Badge => native::BezelStyle::Badge,
            }
        }
    }

    impl WindowStyle {
        pub(crate) fn backend(self) -> native::WindowStyle {
            match self {
                Self::Normal => native::WindowStyle::Normal,
                Self::Panel => native::WindowStyle::Panel,
            }
        }
    }

    impl PopoverEdge {
        pub(crate) fn backend(self) -> native::PopoverEdge {
            match self {
                Self::Leading => native::PopoverEdge::Leading,
                Self::Trailing => native::PopoverEdge::Trailing,
                Self::Top => native::PopoverEdge::Top,
                Self::Bottom => native::PopoverEdge::Bottom,
            }
        }
    }

    impl AlertStyle {
        pub(crate) fn backend(self) -> native::AlertStyle {
            match self {
                Self::Warning => native::AlertStyle::Warning,
                Self::Informational => native::AlertStyle::Informational,
                Self::Critical => native::AlertStyle::Critical,
            }
        }
    }

    impl ScrollAxes {
        pub(crate) fn backend(self) -> native::ScrollAxes {
            native::ScrollAxes {
                horizontal: self.horizontal,
                vertical: self.vertical,
            }
        }
    }
}
