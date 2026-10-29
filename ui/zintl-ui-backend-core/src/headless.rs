//! A backend that keeps rendered nodes without creating native views.

use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
use zintl_ui::view::StoreContext;

use crate::tree::{MainTasks, NodeTree};
use crate::{Context, NodeId, RenderNode};

/// Retains the rendered tree for tests and platforms without a native backend.
///
/// Main-thread tasks queue until [`HeadlessBackend::connect_main_tasks`] runs.
pub struct HeadlessBackend {
    tree: NodeTree<()>,
    main_tasks: MainTasks,
}

impl Default for HeadlessBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadlessBackend {
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

    pub fn value(&self, id: NodeId) -> Option<&RenderNode> {
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

impl RenderBackend<RenderNode> for HeadlessBackend {
    type NodeId = NodeId;

    fn create_context<'a>(&'a self, store_context: StoreContext<'a>) -> Context<'a> {
        Context::new(store_context, Some(self.main_tasks.sender()))
    }

    fn root(&self) -> NodeId {
        NodeTree::<()>::ROOT
    }

    fn create(&mut self, value: &RenderNode, event_route: Option<EventRouteId>) -> NodeId {
        self.tree.create(value, event_route)
    }

    fn update(&mut self, id: NodeId, value: &RenderNode) {
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
