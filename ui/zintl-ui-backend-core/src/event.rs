use std::path::PathBuf;

use zintl_ui::event::Event as EventTrait;

use crate::{CanvasEvent, Color};

/// Selects the event an element route handles.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Activated,
    TextChanged,
    StateChanged,
    ValueChanged,
    SelectionChanged,
    ColorChanged,
    Scrolled,
    Canvas,
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

/// A user change or window lifecycle event reported by a native backend.
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
    /// A scroll view's top-left document point moved.
    Scrolled {
        x: f64,
        y: f64,
    },
    /// Input a canvas received, after its handler saw it.
    Canvas(CanvasEvent),
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
            Self::Scrolled { .. } => EventKind::Scrolled,
            Self::Canvas(_) => EventKind::Canvas,
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
