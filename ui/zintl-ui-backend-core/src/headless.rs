//! A backend that keeps rendered nodes without creating native views.

use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
use zintl_ui::view::StoreContext;

use crate::tree::{MainTasks, NodeTree};
use crate::{NativeRenderNode, NodeId};

/// Retains the rendered tree for tests and platforms without a native backend.
///
/// Main-thread tasks queue until [`HeadlessBackend::connect_main_tasks`] runs.
pub struct HeadlessBackend<R: NativeRenderNode> {
    tree: NodeTree<R, ()>,
    main_tasks: MainTasks,
}

impl<R: NativeRenderNode> Default for HeadlessBackend<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: NativeRenderNode> HeadlessBackend<R> {
    pub fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            main_tasks: MainTasks::new(),
        }
    }

    /// Forwards queued and future main-thread tasks to `sender`.
    pub fn connect_main_tasks(
        &self,
        sender: impl Fn(crate::MainTask) -> bool + Send + Sync + 'static,
    ) {
        self.main_tasks.connect(sender);
    }

    pub fn value(&self, id: NodeId) -> Option<&R> {
        self.tree.value(id)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.tree.children(id)
    }

    /// Returns the Composer route associated with a rendered node.
    pub fn event_route(&self, id: NodeId) -> Option<EventRouteId> {
        self.tree.node(id).event_route
    }
}

impl<R: NativeRenderNode> RenderBackend<R> for HeadlessBackend<R> {
    type NodeId = NodeId;

    fn create_context<'a>(&'a self, store_context: StoreContext<'a>) -> R::Context<'a> {
        R::create_context(store_context, Some(self.main_tasks.sender()))
    }

    fn root(&self) -> NodeId {
        NodeTree::<R, ()>::ROOT
    }

    fn create(&mut self, value: &R, event_route: Option<EventRouteId>) -> NodeId {
        self.tree.create(value, event_route)
    }

    fn update(&mut self, id: NodeId, value: &R) {
        self.tree.node_mut(id).value = Some(value.clone());
    }

    fn set_event_route(&mut self, id: NodeId, event_route: Option<EventRouteId>) {
        self.tree.node_mut(id).event_route = event_route;
    }

    fn insert_child(&mut self, parent: NodeId, index: usize, child: NodeId) {
        self.tree.insert_child(parent, index, child);
    }

    fn remove(&mut self, id: NodeId) {
        self.tree.remove_subtree(id, &mut drop);
    }

    fn move_child(&mut self, parent: NodeId, index: usize, child: NodeId) {
        if !self.tree.is_at(parent, index, child) {
            self.tree.insert_child(parent, index, child);
        }
    }
}
