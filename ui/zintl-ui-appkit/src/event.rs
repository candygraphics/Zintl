use std::path::PathBuf;

use zintl_ui::event::Event as EventTrait;

use crate::Color;

/// Selects the semantic AppKit event handled by an element route.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Activated,
    TextChanged,
    StateChanged,
    ValueChanged,
    SelectionChanged,
    ColorChanged,
    SidebarSelectionChanged,
    SidebarSearchChanged,
    MenuItemSelected,
    PopoverClosed,
    AlertResponse,
    FileDialogCompleted,
    ColorPanelClosed,
    WindowCreated,
    WindowDidResize,
    WindowWillClose,
    WindowDidClose,
}

/// A semantic event produced by the AppKit backend.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Activated,
    TextChanged {
        value: String,
    },
    /// A switch, checkbox, radio button, or toggle button changed.
    StateChanged {
        on: bool,
    },
    /// A slider or number field changed.
    ValueChanged {
        value: f64,
    },
    /// A pop-up button, segmented control, or tab view changed.
    SelectionChanged {
        index: usize,
    },
    ColorChanged {
        color: Color,
    },
    SidebarSelectionChanged {
        id: String,
    },
    SidebarSearchChanged {
        value: String,
    },
    MenuItemSelected {
        id: String,
    },
    PopoverClosed,
    /// `button` is `None` when the alert was dismissed without a choice.
    AlertResponse {
        button: Option<usize>,
    },
    /// `paths` is `None` when the panel was cancelled.
    FileDialogCompleted {
        paths: Option<Vec<PathBuf>>,
    },
    ColorPanelClosed,
    WindowCreated,
    WindowDidResize,
    WindowWillClose,
    WindowDidClose,
}

impl EventTrait for Event {
    type Kind = EventKind;

    fn kind(&self) -> Self::Kind {
        match self {
            Self::Activated => EventKind::Activated,
            Self::TextChanged { .. } => EventKind::TextChanged,
            Self::StateChanged { .. } => EventKind::StateChanged,
            Self::ValueChanged { .. } => EventKind::ValueChanged,
            Self::SelectionChanged { .. } => EventKind::SelectionChanged,
            Self::ColorChanged { .. } => EventKind::ColorChanged,
            Self::SidebarSelectionChanged { .. } => EventKind::SidebarSelectionChanged,
            Self::SidebarSearchChanged { .. } => EventKind::SidebarSearchChanged,
            Self::MenuItemSelected { .. } => EventKind::MenuItemSelected,
            Self::PopoverClosed => EventKind::PopoverClosed,
            Self::AlertResponse { .. } => EventKind::AlertResponse,
            Self::FileDialogCompleted { .. } => EventKind::FileDialogCompleted,
            Self::ColorPanelClosed => EventKind::ColorPanelClosed,
            Self::WindowCreated => EventKind::WindowCreated,
            Self::WindowDidResize => EventKind::WindowDidResize,
            Self::WindowWillClose => EventKind::WindowWillClose,
            Self::WindowDidClose => EventKind::WindowDidClose,
        }
    }
}
