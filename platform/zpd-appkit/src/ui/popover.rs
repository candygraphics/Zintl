use crate::actor::{ActorRef, ActorRollback, EventRouteToken, WindowEventKind};
use crate::geometry::Rect;
use crate::native;
use zpd_objc::Id;

use super::UiError;
use super::callback;
use super::view::{self, ViewRef};

/// The side of the anchor view where a [`Popover`] prefers to appear.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopoverEdge {
    Leading,
    Trailing,
    Top,
    #[default]
    Bottom,
}

impl PopoverEdge {
    /// Converts to `NSRectEdge`, whose Y edges follow the anchor's flippedness.
    fn native(self, flipped: bool) -> u64 {
        match (self, flipped) {
            (Self::Leading, _) => 0,
            (Self::Trailing, _) => 2,
            (Self::Bottom, false) | (Self::Top, true) => 1,
            (Self::Top, false) | (Self::Bottom, true) => 3,
        }
    }
}

/// A transient `NSPopover` anchored to a view.
///
/// The Popover Actor is a child of its anchor, so controls inside it report
/// events through the anchor's Window.
pub struct Popover {
    actor: ActorRef,
    anchor: ActorRef,
    content: ActorRef,
}
impl Popover {
    pub fn new(anchor: ViewRef<'_>) -> Result<Self, UiError> {
        let tree = anchor.actor().tree_handle().ok_or(UiError::Closed)?;
        let popover = native::alloc_init(zpd_objc::class!("NSPopover"));
        let controller = native::alloc_init(zpd_objc::class!("NSViewController"));
        let content = view::new_layout_view(Rect::new(0.0, 0.0, 320.0, 200.0))?;
        // SAFETY: The popover, controller, and content view are live and retained here.
        unsafe {
            zpd_objc::msg_send!(controller.as_ptr(), zpd_objc::sel!("setView:"), ((content.as_ptr()): Id) => ());
            zpd_objc::msg_send!(popover.as_ptr(), zpd_objc::sel!("setContentViewController:"), ((controller.as_ptr()): Id) => ());
            zpd_objc::msg_send!(popover.as_ptr(), zpd_objc::sel!("setBehavior:"), ((native::NS_POPOVER_BEHAVIOR_TRANSIENT): native::Integer) => ());
            zpd_objc::msg_send!(popover.as_ptr(), zpd_objc::sel!("setAnimates:"), ((true): bool) => ());
        }
        let actor = tree.insert_child(anchor.actor(), popover)?;
        let mut rollback = ActorRollback::new(actor.clone());
        tree.add_teardown(&actor, |popover| {
            // SAFETY: Closing first keeps AppKit from presenting a released content tree.
            unsafe {
                zpd_objc::msg_send!(popover, zpd_objc::sel!("close"), () => ());
                zpd_objc::msg_send!(popover, zpd_objc::sel!("setContentViewController:"), ((zpd_objc::NIL): Id) => ());
            }
        })?;
        let controller = tree.insert_child(&actor, controller)?;
        let content = tree.insert_child(&controller, content)?;
        // SAFETY: The content Actor holds a layout view created by `new_layout_view`.
        tree.add_teardown(&content, |view| unsafe {
            view::release_layout_callback(view)
        })?;
        let emit = callback::emitter(&actor);
        callback::install_delegate(&actor, move |_| emit(WindowEventKind::PopoverClosed))?;
        rollback.disarm();
        Ok(Self {
            actor,
            anchor: anchor.actor_ref(),
            content,
        })
    }
    pub fn set_event_route(&self, route: Option<EventRouteToken>) -> Result<(), UiError> {
        self.actor.set_event_route(route).map_err(Into::into)
    }
    /// Returns the view that hosts popover content.
    pub fn content_view(&self) -> ViewRef<'_> {
        ViewRef::from_actor(&self.content)
    }
    pub fn set_content_layout_handler(
        &self,
        callback: impl FnMut(Rect) + 'static,
    ) -> Result<(), UiError> {
        view::set_layout_callback(self.content_view(), callback)
    }
    pub fn set_content_size(&self, width: f64, height: f64) -> Result<(), UiError> {
        let size = native::Size { width, height };
        self.actor
            .with(|popover| {
                // SAFETY: The live object is an NSPopover.
                unsafe {
                    zpd_objc::msg_send!(popover, zpd_objc::sel!("setContentSize:"), ((size): native::Size) => ())
                }
            })
            .map_err(Into::into)
    }
    pub fn is_shown(&self) -> bool {
        self.actor
            .with(|popover| {
                // SAFETY: The live object is an NSPopover.
                unsafe { zpd_objc::msg_send!(popover, zpd_objc::sel!("isShown"), () => bool) }
            })
            .unwrap_or(false)
    }
    pub fn show(&self, edge: PopoverEdge) -> Result<(), UiError> {
        self.actor.with(|popover| {
            self.anchor.with(|anchor| {
                // SAFETY: The anchor is a live view; its bounds position the popover.
                unsafe {
                    let bounds = zpd_objc::msg_send!(anchor, zpd_objc::sel!("bounds"), () => native::Rect);
                    let flipped = zpd_objc::msg_send!(anchor, zpd_objc::sel!("isFlipped"), () => bool);
                    zpd_objc::msg_send!(popover, zpd_objc::sel!("showRelativeToRect:ofView:preferredEdge:"), ((bounds): native::Rect, (anchor): Id, (edge.native(flipped)): u64) => ());
                }
            })
        })??;
        Ok(())
    }
    pub fn close(&self) -> Result<(), UiError> {
        self.actor
            .with(|popover| {
                // SAFETY: The live object is an NSPopover.
                unsafe { zpd_objc::msg_send!(popover, zpd_objc::sel!("performClose:"), ((zpd_objc::NIL): Id) => ()) }
            })
            .map_err(Into::into)
    }
}
impl Drop for Popover {
    fn drop(&mut self) {
        self.actor.remove();
    }
}
