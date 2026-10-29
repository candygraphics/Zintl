use std::rc::Rc;

use zpd_winui3::{AsElement, Control, Dialog, Popover, Window};

use super::native::{CanvasHost, NativeNode};

/// Native state of a node presented outside its parent's layout.
pub(super) enum Attachment {
    /// Clears the anchor's context menu when dropped.
    ContextMenu {
        _menu: ContextMenu,
    },
    Popover {
        popover: Popover,
        content: Control,
        host: Rc<CanvasHost>,
    },
    /// Hides the dialog when dropped.
    Alert {
        _dialog: Dialog,
    },
    /// A file picker cannot be dismissed, so only its completion is awaited.
    FileDialog,
    ColorPanel {
        _window: Window,
        picker: Control,
    },
}

pub(super) struct ContextMenu {
    pub(super) anchor: Rc<NativeNode>,
}

impl Drop for ContextMenu {
    fn drop(&mut self) {
        let _ = self.anchor.control.clear_context_menu();
    }
}
