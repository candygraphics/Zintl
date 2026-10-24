use std::cell::RefCell;
use std::rc::Rc;

use crate::actor::{ActorRef, EventRouteToken, WindowEventKind};
use zpd_objc::Id;

use super::commands::{self, CommandItem, MenuCallback};
use super::view::ViewRef;
use super::{UiError, callback};

/// A contextual `NSMenu` attached to a view.
///
/// Choosing a custom item emits [`WindowEventKind::MenuItemSelected`] from
/// the menu Actor, which carries its own event route.
pub struct ContextMenu {
    actor: ActorRef,
}
impl ContextMenu {
    pub fn install(
        view: ViewRef<'_>,
        items: &[CommandItem],
        route: Option<EventRouteToken>,
    ) -> Result<Self, UiError> {
        let tree = view.actor().tree_handle().ok_or(UiError::Closed)?;
        let menu_actor: Rc<RefCell<Option<ActorRef>>> = Rc::new(RefCell::new(None));
        let source = menu_actor.clone();
        let selected: MenuCallback = Rc::new(RefCell::new(move |id: &str| {
            let actor = source.borrow().clone();
            if let Some(actor) = actor {
                callback::emitter(&actor)(WindowEventKind::MenuItemSelected { id: id.into() });
            }
        }));
        let actor = commands::build_menu(&tree, view.actor(), "", items, &selected)?;
        *menu_actor.borrow_mut() = Some(actor.clone());
        actor.set_event_route(route)?;
        let owner = view.actor_ref();
        tree.add_teardown(&actor, move |menu| {
            // Only clear the view's menu if this menu is still installed.
            let _ = owner.with(|view| {
                // SAFETY: Both objects are live while this teardown runs.
                unsafe {
                    if zpd_objc::msg_send!(view, zpd_objc::sel!("menu"), () => Id) == menu {
                        zpd_objc::msg_send!(view, zpd_objc::sel!("setMenu:"), ((zpd_objc::NIL): Id) => ());
                    }
                }
            });
        })?;
        view.with(|view| {
            actor.with(|menu| {
                // SAFETY: The view retains its contextual menu.
                unsafe { zpd_objc::msg_send!(view, zpd_objc::sel!("setMenu:"), ((menu): Id) => ()) }
            })
        })??;
        Ok(Self { actor })
    }
    pub fn set_event_route(&self, route: Option<EventRouteToken>) -> Result<(), UiError> {
        self.actor.set_event_route(route).map_err(Into::into)
    }
}
impl Drop for ContextMenu {
    fn drop(&mut self) {
        self.actor.remove();
    }
}
