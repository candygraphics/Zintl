//! A headless app for view tests.

use zintl_ui::composer::Composer;
use zintl_ui::element::IntoElement;
use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
use zintl_ui::store::Store;
use zintl_ui_backend_core::HeadlessBackend;

use crate::{Event, NodeId, RenderNode};

/// A render node with the nodes below it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderedNode {
    pub(crate) value: RenderNode,
    pub(crate) children: Vec<RenderedNode>,
}

/// Views mounted on the headless backend, read back as render nodes.
pub(crate) struct TestApp {
    pub(crate) composer: Composer<RenderNode, HeadlessBackend>,
}

impl TestApp {
    pub(crate) fn new<E>(root: E) -> Self
    where
        E: IntoElement<Output = RenderNode>,
    {
        let mut composer = Composer::new(HeadlessBackend::new());
        composer.mount(root);
        Self { composer }
    }

    /// Returns the first root-level node.
    pub(crate) fn render(&self) -> RenderNode {
        self.render_tree().value
    }

    /// Returns the first root-level node with the nodes below it.
    pub(crate) fn render_tree(&self) -> RenderedNode {
        self.render_roots().remove(0)
    }

    /// Renders every root-level element, such as a main menu and windows.
    pub(crate) fn render_roots(&self) -> Vec<RenderedNode> {
        let backend = self.composer.backend();
        backend
            .children(backend.root())
            .iter()
            .map(|child| Self::rendered_node(backend, *child))
            .collect()
    }

    fn rendered_node(backend: &HeadlessBackend, id: NodeId) -> RenderedNode {
        RenderedNode {
            value: backend
                .value(id)
                .cloned()
                .expect("render nodes always have values"),
            children: backend
                .children(id)
                .iter()
                .map(|child| Self::rendered_node(backend, *child))
                .collect(),
        }
    }

    pub(crate) fn dispatch_root_event(&mut self, event: Event) -> bool {
        self.dispatch_at(&[0], event)
    }

    /// Dispatches to the node at `path`, starting with the root's children.
    pub(crate) fn dispatch_at(&mut self, path: &[usize], event: Event) -> bool {
        let route = Self::event_route_at(self.composer.backend(), path)
            .expect("the addressed node must have an event route");
        self.composer.dispatch_event(route, event)
    }

    fn event_route_at(backend: &HeadlessBackend, path: &[usize]) -> Option<EventRouteId> {
        let mut node = backend.root();
        for index in path {
            node = *backend.children(node).get(*index)?;
        }
        backend.event_route(node)
    }

    pub(crate) fn get<T: Clone + 'static>(&mut self, store: Store<T>) -> T {
        self.composer.context(|cx| cx.get(store).clone())
    }

    pub(crate) fn set<T: 'static>(&mut self, store: Store<T>, value: T) {
        self.composer
            .context(|cx| cx.update(store, |current| *current = value));
        self.composer.flush();
    }
}
