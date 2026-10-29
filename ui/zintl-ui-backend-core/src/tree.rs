//! Platform-neutral storage shared by native backends.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};

use zintl_ui::event::EventRouteId;

use crate::{
    CommandItem, CommandModifier, CommandRole, CommandSet, MainTask, MainTaskSender, NodeId,
    RenderNode, WindowAppMenu, WindowStyle,
};

/// A rendered node and the backend's native state `S` for it.
pub struct TreeNode<S> {
    pub value: Option<RenderNode>,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub event_route: Option<EventRouteId>,
    pub state: S,
}

/// Rendered nodes addressed by [`NodeId`]; the root has no value.
pub struct NodeTree<S> {
    nodes: Vec<Option<TreeNode<S>>>,
}

impl<S: Default> Default for NodeTree<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Default> NodeTree<S> {
    pub const ROOT: NodeId = NodeId(0);

    pub fn new() -> Self {
        Self {
            nodes: vec![Some(Self::new_node(None, None))],
        }
    }

    fn new_node(value: Option<RenderNode>, event_route: Option<EventRouteId>) -> TreeNode<S> {
        TreeNode {
            value,
            parent: None,
            children: Vec::new(),
            event_route,
            state: S::default(),
        }
    }

    pub fn create(&mut self, value: &RenderNode, event_route: Option<EventRouteId>) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes
            .push(Some(Self::new_node(Some(value.clone()), event_route)));
        id
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.get(id.0).is_some_and(Option::is_some)
    }

    pub fn node(&self, id: NodeId) -> &TreeNode<S> {
        self.nodes
            .get(id.0)
            .and_then(Option::as_ref)
            .expect("node handle must belong to this backend")
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut TreeNode<S> {
        self.nodes
            .get_mut(id.0)
            .and_then(Option::as_mut)
            .expect("node handle must belong to this backend")
    }

    pub fn value(&self, id: NodeId) -> Option<&RenderNode> {
        self.nodes
            .get(id.0)
            .and_then(Option::as_ref)
            .and_then(|node| node.value.as_ref())
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.node(id).children
    }

    pub fn kind(&self, id: NodeId) -> RenderNode {
        self.value(id).expect("render nodes have values").clone()
    }

    pub fn states_mut(&mut self) -> impl Iterator<Item = &mut S> {
        self.nodes.iter_mut().flatten().map(|node| &mut node.state)
    }

    pub fn detach(&mut self, child: NodeId) {
        let Some(parent) = self.node(child).parent else {
            return;
        };
        self.node_mut(parent)
            .children
            .retain(|candidate| *candidate != child);
        self.node_mut(child).parent = None;
    }

    pub fn insert_child(&mut self, parent: NodeId, index: usize, child: NodeId) {
        self.detach(child);
        let index = index.min(self.node(parent).children.len());
        self.node_mut(parent).children.insert(index, child);
        self.node_mut(child).parent = Some(parent);
    }

    /// Returns whether `child` already sits at `index` in `parent`.
    pub fn is_at(&self, parent: NodeId, index: usize, child: NodeId) -> bool {
        self.node(child).parent == Some(parent)
            && self.node(parent).children.get(index) == Some(&child)
    }

    /// Removes `id` and its descendants, passing each removed node to
    /// `release` after its children.
    pub fn remove_subtree(&mut self, id: NodeId, release: &mut impl FnMut(TreeNode<S>)) {
        if !self.contains(id) {
            return;
        }
        for child in self.node(id).children.clone() {
            self.remove_subtree(child, release);
        }
        self.detach(id);
        if let Some(node) = self.nodes.get_mut(id.0).and_then(Option::take) {
            release(node);
        }
    }

    pub fn root_children(&self) -> impl Iterator<Item = (NodeId, RenderNode)> + '_ {
        self.children(Self::ROOT)
            .iter()
            .map(|id| (*id, self.kind(*id)))
    }

    /// Returns declared windows whose state `closed` does not report closed.
    pub fn window_ids(&self, closed: impl Fn(&S) -> bool) -> Vec<NodeId> {
        self.root_children()
            .filter(|(id, kind)| {
                matches!(kind, RenderNode::Window { .. }) && !closed(&self.node(*id).state)
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Returns whether a normal window is declared and not closed.
    pub fn has_normal_window(&self, closed: impl Fn(&S) -> bool) -> bool {
        self.root_children().any(|(id, kind)| {
            matches!(
                kind,
                RenderNode::Window {
                    style: WindowStyle::Normal,
                    ..
                }
            ) && !closed(&self.node(id).state)
        })
    }

    pub fn main_menu_route(&self) -> Option<EventRouteId> {
        self.root_children()
            .find(|(_, kind)| matches!(kind, RenderNode::MainMenu(_)))
            .and_then(|(id, _)| self.node(id).event_route)
    }

    /// Returns the declared main menu, else `fallback`, else a Quit item.
    pub fn main_menu(&self, fallback: Option<&CommandSet>) -> CommandSet {
        let declared = self.root_children().find_map(|(_, kind)| match kind {
            RenderNode::MainMenu(commands) => Some(commands),
            _ => None,
        });
        declared.or_else(|| fallback.cloned()).unwrap_or_else(|| {
            let mut quit = CommandItem::new("Quit");
            quit.role = Some(CommandRole::Quit);
            quit.key = Some("q".into());
            quit.modifiers = vec![CommandModifier::Cmd];
            CommandSet {
                app_menu: Some(WindowAppMenu { items: vec![quit] }),
                menus: Vec::new(),
            }
        })
    }
}

impl RenderNode {
    /// Returns whether a presentable attachment is declared as shown.
    pub fn is_presented(&self) -> bool {
        matches!(
            self,
            Self::Popover {
                presented: true,
                ..
            } | Self::Alert {
                presented: true,
                ..
            } | Self::FileDialog {
                presented: true,
                ..
            } | Self::ColorPanel {
                presented: true,
                ..
            }
        )
    }

    /// Returns whether this is a window-modal sheet declared as shown.
    pub fn is_presented_sheet(&self) -> bool {
        matches!(
            self,
            Self::Alert {
                presented: true,
                ..
            } | Self::FileDialog {
                presented: true,
                ..
            }
        )
    }
}

/// Where main-thread tasks go before and after the message loop starts.
enum MainTaskRoute {
    Pending(Vec<MainTask>),
    Connected(Box<MainTaskSender>),
}

/// Queues main-thread tasks until a message loop connects a sender.
pub struct MainTasks {
    route: Arc<Mutex<MainTaskRoute>>,
    sender: Box<MainTaskSender>,
}

impl Default for MainTasks {
    fn default() -> Self {
        Self::new()
    }
}

impl MainTasks {
    pub fn new() -> Self {
        let route = Arc::new(Mutex::new(MainTaskRoute::Pending(Vec::new())));
        let target = route.clone();
        // Tasks queued during the initial mount wait until the loop connects.
        let sender = Box::new(move |task: MainTask| {
            match &mut *target.lock().unwrap_or_else(PoisonError::into_inner) {
                MainTaskRoute::Pending(tasks) => {
                    tasks.push(task);
                    true
                }
                MainTaskRoute::Connected(sender) => sender(task),
            }
        });
        Self { route, sender }
    }

    pub fn sender(&self) -> &MainTaskSender {
        &*self.sender
    }

    /// Forwards queued and future tasks to `sender`.
    pub fn connect(&self, sender: impl Fn(MainTask) -> bool + Send + Sync + 'static) {
        let mut route = self.route.lock().unwrap_or_else(PoisonError::into_inner);
        if let MainTaskRoute::Pending(tasks) = &mut *route {
            for task in tasks.drain(..) {
                sender(task);
            }
        }
        *route = MainTaskRoute::Connected(Box::new(sender));
    }
}

/// A shared slot keeping the first error raised by native callbacks.
pub struct ErrorSlot<E>(Rc<RefCell<Option<E>>>);

impl<E> Clone for ErrorSlot<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<E> Default for ErrorSlot<E> {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(None)))
    }
}

impl<E> ErrorSlot<E> {
    /// Keeps `error` unless an earlier one is pending.
    pub fn record(&self, error: E) {
        let mut slot = self.0.borrow_mut();
        if slot.is_none() {
            *slot = Some(error);
        }
    }

    pub fn take(&self) -> Result<(), E> {
        self.0.borrow_mut().take().map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn main_tasks_wait_until_a_sender_connects() {
        // Verifies tasks queued before connection run in order once a loop connects.
        let tasks = MainTasks::new();
        let count = Arc::new(AtomicUsize::new(0));
        let queued = count.clone();
        assert!((tasks.sender())(Box::new(move || {
            queued.fetch_add(1, Ordering::Relaxed);
        })));
        assert_eq!(count.load(Ordering::Relaxed), 0);
        tasks.connect(|task| {
            task();
            true
        });
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn error_slot_keeps_the_first_error() {
        // Verifies later callback errors do not replace the first pending error.
        let slot = ErrorSlot::default();
        slot.record(1);
        slot.clone().record(2);
        assert_eq!(slot.take(), Err(1));
        assert_eq!(slot.take(), Ok(()));
    }
}
