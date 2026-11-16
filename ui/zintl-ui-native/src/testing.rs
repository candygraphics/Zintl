//! A headless harness for inspecting views in tests.

use zintl_ui::composer::Composer;
use zintl_ui::element::IntoElement;
use zintl_ui::renderer::RenderBackend;
use zintl_ui::store::{Store, Trigger};
use zintl_ui_backend_core::HeadlessBackend;

use crate::{Context, Event, NodeId, RenderNode};

/// A render node with the nodes below it.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedNode {
    pub value: RenderNode,
    pub children: Vec<RenderedNode>,
}

/// Mounts views on a headless backend to read their render tree and send them events.
pub struct Harness {
    pub(crate) composer: Composer<RenderNode, HeadlessBackend>,
}

impl Harness {
    /// Mounts `root`.
    pub fn new<E>(root: E) -> Self
    where
        E: IntoElement<Output = RenderNode>,
    {
        let mut harness = Self::empty();
        harness.mount(root);
        harness
    }

    /// Creates a harness with nothing mounted, to create Stores before [`Harness::mount`].
    pub fn empty() -> Self {
        Self {
            composer: Composer::new(HeadlessBackend::new()),
        }
    }

    /// Mounts `root` as the next root-level element.
    pub fn mount<E>(&mut self, root: E)
    where
        E: IntoElement<Output = RenderNode>,
    {
        self.composer.mount(root);
    }

    /// Returns the first root-level element with the nodes below it.
    pub fn tree(&self) -> RenderedNode {
        let root = self.node(&[0]).expect("a view must be mounted");
        self.rendered(root)
    }

    /// Returns every root-level element, such as a main menu and windows.
    pub fn roots(&self) -> Vec<RenderedNode> {
        let backend = self.composer.backend();
        backend
            .children(backend.root())
            .iter()
            .map(|child| self.rendered(*child))
            .collect()
    }

    /// Returns the first view or window with the ID `id`, parents before children.
    pub fn find(&self, id: &str) -> Option<RenderedNode> {
        self.node_with_id(id).map(|node| self.rendered(node))
    }

    /// Returns whether the view at `path` and every view above it are enabled.
    ///
    /// `path` lists child indexes, starting with the root-level elements.
    pub fn is_enabled(&self, path: &[usize]) -> bool {
        let node = self.node(path).expect("the path must address a node");
        self.composer.backend().is_enabled(node)
    }

    /// Sends `event` to the node at `path` and returns whether a handler ran.
    ///
    /// `path` lists child indexes, starting with the root-level elements.
    pub fn dispatch(&mut self, path: &[usize], event: Event) -> bool {
        let node = self.node(path).expect("the path must address a node");
        self.dispatch_node(node, event)
    }

    /// Sends `event` to the first view or window with the ID `id` and returns whether a handler ran.
    pub fn dispatch_to(&mut self, id: &str, event: Event) -> bool {
        let node = self
            .node_with_id(id)
            .unwrap_or_else(|| panic!("no view has the ID {id:?}"));
        self.dispatch_node(node, event)
    }

    /// Returns a copy of the value in `store`.
    pub fn get<T: Clone + 'static>(&mut self, store: Store<T>) -> T {
        self.composer.context(|cx| cx.get(store).clone())
    }

    /// Writes `value` to `store` and renders the views that depend on it.
    pub fn set<T: 'static>(&mut self, store: Store<T>, value: T) {
        self.update(store, |current| *current = value);
    }

    /// Fires `trigger` with `value` and renders the views that obey it.
    pub fn fire<T: 'static>(&mut self, trigger: Trigger<T>, value: T) {
        self.composer.context(|cx| cx.fire(trigger, value));
        self.composer.flush();
    }

    /// Updates `store` and renders the views that depend on it.
    pub fn update<T: 'static>(&mut self, store: Store<T>, update: impl FnOnce(&mut T)) {
        self.composer.context(|cx| cx.update(store, update));
        self.composer.flush();
    }

    /// Creates a Store outside `View::init`.
    pub fn store<T: 'static>(&mut self, value: T) -> Store<T> {
        self.composer.context(|cx| cx.store(value))
    }

    /// Runs `action` with the context views receive, for anything the other methods do not cover.
    pub fn context<R>(&mut self, action: impl FnOnce(&mut Context<'_>) -> R) -> R {
        self.composer.context(action)
    }

    fn dispatch_node(&mut self, node: NodeId, event: Event) -> bool {
        let route = self
            .composer
            .backend()
            .event_route(node)
            .expect("the addressed node must handle events");
        self.composer.dispatch_event(route, event)
    }

    fn node(&self, path: &[usize]) -> Option<NodeId> {
        let backend = self.composer.backend();
        let mut node = backend.root();
        for index in path {
            node = *backend.children(node).get(*index)?;
        }
        Some(node)
    }

    fn node_with_id(&self, id: &str) -> Option<NodeId> {
        let backend = self.composer.backend();
        let mut pending = vec![backend.root()];
        while let Some(node) = pending.pop() {
            let found = match backend.value(node) {
                Some(RenderNode::View {
                    id: Some(found), ..
                })
                | Some(RenderNode::Window {
                    id: Some(found), ..
                }) => found == id,
                _ => false,
            };
            if found {
                return Some(node);
            }
            // The stack pops the last entry, so children go in reversed to visit them in order.
            pending.extend(backend.children(node).iter().rev());
        }
        None
    }

    fn rendered(&self, node: NodeId) -> RenderedNode {
        let backend = self.composer.backend();
        RenderedNode {
            value: backend
                .value(node)
                .cloned()
                .expect("render nodes always have values"),
            children: backend
                .children(node)
                .iter()
                .map(|child| self.rendered(*child))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attach, Button, Rect, Text, VStack, Window, list};

    #[test]
    fn find_returns_a_nested_view_by_its_id() {
        // Verifies IDs are found below windows and stacks, and unknown IDs are not.
        let harness = Harness::new(
            Window::new(Rect::new(0.0, 0.0, 320.0, 240.0), "Main")
                .id("main")
                .content(VStack::new(list![
                    Text::new("Title"),
                    VStack::new(list![Text::new("Detail").id("detail")]).id("group"),
                ])),
        );

        assert!(matches!(
            harness.find("main").map(|node| node.value),
            Some(RenderNode::Window { .. })
        ));
        assert_eq!(
            harness.find("group").map(|node| node.children.len()),
            Some(1)
        );
        assert_eq!(
            harness.find("detail"),
            Some(harness.tree().children[0].children[1].children[0].clone())
        );
        assert_eq!(harness.find("missing"), None);
    }

    #[test]
    fn dispatch_to_reaches_the_view_with_the_id() {
        // Verifies an event sent by ID runs that view's handler and not a sibling's.
        let mut harness = Harness::empty();
        let clicked = harness.store(String::new());
        let press = |name: &'static str| {
            Button::new(name)
                .on_click(move |cx| cx.update(clicked, |clicked| *clicked = name.to_owned()))
                .id(name)
        };
        harness.mount(VStack::new(list![
            press("first"),
            VStack::new(list![press("second")])
        ]));

        assert!(harness.dispatch_to("second", Event::Activated));
        assert_eq!(harness.get(clicked), "second");
        assert!(harness.dispatch(&[0, 0], Event::Activated));
        assert_eq!(harness.get(clicked), "first");
    }
}
