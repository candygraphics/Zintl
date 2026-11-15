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

    /// Returns the rendered node that `route` is attached to.
    pub fn find_route(&self, route: EventRouteId) -> Option<NodeId> {
        self.nodes.iter().enumerate().find_map(|(index, node)| {
            node.as_ref()
                .is_some_and(|node| node.event_route == Some(route))
                .then_some(NodeId(index))
        })
    }

    /// Returns the controls whose rendered value is shown again after the user changed `changed`.
    ///
    /// The composer skips updates whose value did not change, so a change the
    /// app rejected would otherwise stay on screen. Siblings are included,
    /// because choosing a radio button turns off the others in its group.
    pub fn user_change_targets(&self, changed: &[NodeId]) -> Vec<NodeId> {
        let reverts = |id: NodeId| {
            matches!(
                self.value(id),
                Some(RenderNode::View { kind, .. }) if kind.reverts_user_changes()
            )
        };
        let mut targets = Vec::new();
        for id in changed
            .iter()
            .filter(|id| self.contains(**id) && reverts(**id))
        {
            let siblings = self
                .node(*id)
                .parent
                .map_or(std::slice::from_ref(id), |parent| self.children(parent));
            for sibling in siblings {
                if reverts(*sibling) && !targets.contains(sibling) {
                    targets.push(*sibling);
                }
            }
        }
        targets
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

    /// Returns whether the view at `id` takes input: it and every view above it are enabled.
    ///
    /// Windows and presentations such as popovers start a new scope.
    pub fn is_enabled(&self, id: NodeId) -> bool {
        let mut current = Some(id);
        while let Some(id) = current {
            let node = self.node(id);
            match &node.value {
                Some(RenderNode::View { enabled: false, .. }) => return false,
                Some(RenderNode::View { .. }) => current = node.parent,
                _ => break,
            }
        }
        true
    }

    /// Returns `id` and its descendants, parents before their children.
    pub fn subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut nodes = vec![id];
        let mut next = 0;
        while let Some(node) = nodes.get(next).copied() {
            nodes.extend_from_slice(self.children(node));
            next += 1;
        }
        nodes
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
    sender: Arc<MainTaskSender>,
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
        let sender = Arc::new(move |task: MainTask| {
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

    /// Returns an owned sender, for handles that outlive a borrow such as canvas invalidators.
    pub fn shared_sender(&self) -> Arc<MainTaskSender> {
        self.sender.clone()
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

    /// Builds trees of plain containers.
    struct Containers;

    impl Containers {
        fn node(enabled: bool) -> RenderNode {
            RenderNode::View {
                kind: crate::ViewKind::Container,
                layout: zintl_ui_layout::LayoutStyle::leaf(Default::default()),
                id: None,
                enabled,
                title_bar_interactive: false,
                help: None,
                accessibility: Default::default(),
            }
        }

        /// Creates a container under `parent`.
        fn add(tree: &mut NodeTree<()>, parent: NodeId, enabled: bool) -> NodeId {
            let id = tree.create(&Self::node(enabled), None);
            let index = tree.children(parent).len();
            tree.insert_child(parent, index, id);
            id
        }
    }

    #[test]
    fn views_inside_a_disabled_view_are_disabled() {
        // Verifies a view is enabled only while it and every view above it are.
        let mut tree = NodeTree::<()>::new();
        let disabled = Containers::add(&mut tree, NodeTree::<()>::ROOT, false);
        let child = Containers::add(&mut tree, disabled, true);
        let grandchild = Containers::add(&mut tree, child, true);
        let sibling = Containers::add(&mut tree, NodeTree::<()>::ROOT, true);

        assert!(!tree.is_enabled(disabled));
        assert!(!tree.is_enabled(child));
        assert!(!tree.is_enabled(grandchild));
        assert!(tree.is_enabled(sibling));
    }

    #[test]
    fn subtree_lists_parents_before_their_children() {
        // Verifies a subtree walk covers every descendant and nothing outside it.
        let mut tree = NodeTree::<()>::new();
        let parent = Containers::add(&mut tree, NodeTree::<()>::ROOT, true);
        let first = Containers::add(&mut tree, parent, true);
        let second = Containers::add(&mut tree, parent, true);
        let nested = Containers::add(&mut tree, first, true);
        Containers::add(&mut tree, NodeTree::<()>::ROOT, true);

        assert_eq!(tree.subtree(parent), vec![parent, first, second, nested]);
    }

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

    #[test]
    fn find_route_returns_the_node_with_that_route() {
        // Verifies events can be traced back to the node their route was attached to.
        let mut tree = NodeTree::<()>::new();
        let menu = RenderNode::ContextMenu { items: Vec::new() };
        let route = EventRouteId::from_raw(7);
        tree.create(&menu, None);
        let routed = tree.create(&menu, Some(route));
        assert_eq!(tree.find_route(route), Some(routed));
        assert_eq!(tree.find_route(EventRouteId::from_raw(8)), None);
    }

    /// Returns a view node of `kind` for tree tests.
    fn view(kind: crate::ViewKind) -> RenderNode {
        RenderNode::View {
            kind,
            layout: zintl_ui_layout::LayoutStyle::leaf(zintl_ui_layout::Size::default()),
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        }
    }

    /// Returns a button node of `button_type` for tree tests.
    fn button(button_type: crate::ButtonType) -> RenderNode {
        view(crate::ViewKind::Button(crate::Button {
            title: String::new(),
            button_type,
            bezel: None,
            on: false,
            system_image: None,
            key_equivalent: None,
            destructive: false,
            shows_menu: false,
            keeps_state: false,
            title_color: None,
        }))
    }

    #[test]
    fn user_changes_revert_discrete_siblings_only() {
        // Verifies a changed radio button reverts its group, but not text fields or push buttons.
        let mut tree = NodeTree::<()>::new();
        let group = tree.create(&view(crate::ViewKind::Container), None);
        let first = tree.create(&button(crate::ButtonType::Radio), None);
        let second = tree.create(&button(crate::ButtonType::Radio), None);
        let push = tree.create(&button(crate::ButtonType::MomentaryPushIn), None);
        let search = tree.create(
            &view(crate::ViewKind::SearchField {
                value: String::new(),
                placeholder: None,
            }),
            None,
        );
        for (index, child) in [first, second, push, search].into_iter().enumerate() {
            tree.insert_child(group, index, child);
        }
        assert_eq!(tree.user_change_targets(&[second]), [first, second]);
        // A text field the user typed into keeps its text.
        assert!(tree.user_change_targets(&[search]).is_empty());
    }

    #[test]
    fn only_tables_that_follow_their_selection_revert_it() {
        // Verifies a bound table shows its Store again after a rejected click, but a free one keeps the click.
        let table = |follows_selection| {
            view(crate::ViewKind::Table {
                columns: Vec::new(),
                rows: Vec::new(),
                selected: None,
                follows_selection,
                sort: None,
                fit_columns: false,
                striped: false,
            })
        };
        let mut tree = NodeTree::<()>::new();
        let bound = tree.create(&table(true), None);
        let free = tree.create(&table(false), None);
        assert_eq!(tree.user_change_targets(&[bound]), [bound]);
        assert!(tree.user_change_targets(&[free]).is_empty());
    }
}
