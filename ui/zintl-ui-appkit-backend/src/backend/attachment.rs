use std::cell::RefCell;
use std::rc::Rc;

use zpd_appkit::actor::EventRouteToken;
use zpd_appkit::ui::{
    AlertSheet, ColorPanelSession, ContextMenu, FileDialogSheet, Popover, UiError,
};

use super::layout::HostLayout;

/// Native state of a node presented outside its parent's layout.
pub(super) enum Attachment {
    ContextMenu(ContextMenu),
    Popover {
        popover: Popover,
        layout: Rc<RefCell<HostLayout>>,
    },
    Alert(Option<AlertSheet>),
    FileDialog(Option<FileDialogSheet>),
    ColorPanel(Option<ColorPanelSession>),
}

impl Attachment {
    pub(super) fn set_event_route(&self, route: Option<EventRouteToken>) -> Result<(), UiError> {
        match self {
            Self::ContextMenu(menu) => menu.set_event_route(route),
            Self::Popover { popover, .. } => popover.set_event_route(route),
            Self::Alert(Some(sheet)) => sheet.set_event_route(route),
            Self::FileDialog(Some(sheet)) => sheet.set_event_route(route),
            Self::ColorPanel(Some(session)) => session.set_event_route(route),
            Self::Alert(None) | Self::FileDialog(None) | Self::ColorPanel(None) => Ok(()),
        }
    }
}
