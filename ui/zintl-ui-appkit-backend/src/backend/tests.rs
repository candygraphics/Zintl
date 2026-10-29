use std::sync::mpsc;

use zintl_ui::composer::Composer;
use zintl_ui::element::{Element, IntoElement};
use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
use zintl_ui::view::View;
use zintl_ui_layout::{LayoutStyle, Size};
use zpd_appkit::actor::WindowEventKind;

use super::app::WindowEventAction;
use super::layout::native_frame;
use super::*;

/// Builds the render nodes the tests insert.
struct Nodes;

impl Nodes {
    fn container() -> RenderNode {
        RenderNode::View {
            kind: ViewKind::Container,
            layout: LayoutStyle::leaf(Size::new(0.0, 0.0)),
            id: None,
            enabled: true,
        }
    }

    fn window(style: WindowStyle) -> RenderNode {
        RenderNode::Window {
            sidebar: None,
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            title: String::new(),
            full_size_content_view: false,
            id: None,
            style,
        }
    }
}

#[test]
fn converts_taffy_top_origin_to_cgrect_bottom_origin() {
    // Verifies AppKit frames flip Taffy's vertical coordinate within the parent view.
    let frame = zintl_ui_layout::Rect {
        x: 12.0,
        y: 20.0,
        width: 80.0,
        height: 32.0,
    };

    assert_eq!(
        native_frame(frame, Some(100.0)),
        zpd_appkit::geometry::Rect::new(12.0, 48.0, 80.0, 32.0)
    );
}

#[test]
fn flipped_parents_keep_the_top_origin() {
    // Verifies scroll documents, which are flipped, receive Taffy's y coordinate unchanged.
    let frame = zintl_ui_layout::Rect {
        x: 0.0,
        y: 20.0,
        width: 80.0,
        height: 32.0,
    };

    assert_eq!(
        native_frame(frame, None),
        zpd_appkit::geometry::Rect::new(0.0, 20.0, 80.0, 32.0)
    );
}

#[test]
fn removing_backend_parent_removes_its_descendants() {
    // Verifies renderer removal cannot leave detached descendants in backend storage.
    let mut backend = AppKitBackend::new();
    let parent = backend.create(&Nodes::container(), None);
    let child = backend.create(&Nodes::container(), None);
    backend.insert_child(backend.root(), 0, parent);
    backend.insert_child(parent, 0, child);

    backend.remove(parent);

    assert!(!backend.tree.contains(parent));
    assert!(!backend.tree.contains(child));
    assert!(backend.children(backend.root()).is_empty());
}

#[test]
fn backend_tracks_composer_event_routes_on_nodes() {
    // Verifies the adapter stores direct routes instead of building an Actor-to-UI lookup.
    let mut backend = AppKitBackend::new();
    let route = EventRouteId::from_raw(17);
    let node = backend.create(&Nodes::container(), Some(route));
    assert_eq!(backend.event_route(node), Some(route));

    backend.set_event_route(node, None);
    assert_eq!(backend.event_route(node), None);
}

#[test]
fn resize_event_uses_normal_synchronization() {
    // Verifies resize events remain dispatchable without a separate Taffy layout path.
    assert_eq!(
        WindowEventAction::of(&WindowEventKind::DidResize),
        WindowEventAction::Synchronize
    );
}

#[test]
fn closing_window_waits_for_did_close_before_synchronizing() {
    // Verifies WillClose skips synchronization and DidClose reports a closed window.
    assert_eq!(
        WindowEventAction::of(&WindowEventKind::WillClose),
        WindowEventAction::WaitForClose
    );
    assert_eq!(
        WindowEventAction::of(&WindowEventKind::DidClose),
        WindowEventAction::Closed
    );
    assert_eq!(
        WindowEventAction::of(&WindowEventKind::Created),
        WindowEventAction::Synchronize
    );
}

#[test]
fn only_normal_windows_keep_the_app_running() {
    // Verifies panels alone do not keep the app alive once a window has been shown.
    let mut backend = AppKitBackend::new();
    let panel = backend.create(&Nodes::window(WindowStyle::Panel), None);
    backend.insert_child(backend.root(), 0, panel);
    assert!(!backend.should_terminate());

    backend.opened_window = true;
    assert!(backend.should_terminate());

    let window = backend.create(&Nodes::window(WindowStyle::Normal), None);
    backend.insert_child(backend.root(), 1, window);
    assert!(!backend.should_terminate());
}

#[test]
fn attachments_are_excluded_from_layout_kinds() {
    // Verifies only presentation nodes are kept out of their parent's layout.
    assert!(
        RenderNode::ContextMenu { items: Vec::new() }.is_attachment()
            && RenderNode::Popover {
                presented: false,
                edge: crate::PopoverEdge::Bottom,
                size: None,
            }
            .is_attachment()
    );
    assert!(!Nodes::container().is_attachment());
    assert!(!RenderNode::MainMenu(CommandSet::default()).is_attachment());
}

struct PerformMainView;

impl View for PerformMainView {
    type Output = RenderNode;

    fn render<'a>(
        &self,
        cx: &mut <RenderNode as zintl_ui::renderer::RenderNode>::Context<'a>,
    ) -> impl IntoElement<Output = Self::Output> {
        cx.perform_main(|| {});
        Element::node(Nodes::container())
    }
}

#[test]
fn backend_context_queues_main_thread_tasks() {
    // The AppKit backend injects its sender into the RenderNode-associated Context.
    let (sender, receiver) = mpsc::channel();
    let mut backend = AppKitBackend::new();
    backend.set_main_task_sender(move |task| sender.send(task).is_ok());
    let mut composer = Composer::new(backend);

    composer.mount(PerformMainView);

    receiver.recv().unwrap()();
}

#[test]
fn tasks_queued_before_loop_start_are_forwarded_on_connect() {
    // Verifies a render before the message loop starts can still queue main-thread work.
    let mut composer = Composer::new(AppKitBackend::new());
    composer.mount(PerformMainView);

    let (sender, receiver) = mpsc::channel();
    composer
        .backend_mut()
        .set_main_task_sender(move |task| sender.send(task).is_ok());

    receiver.try_recv().unwrap()();
}

#[test]
fn unchanged_declarative_bounds_are_not_reapplied() {
    // Verifies later synchronization preserves a user-resized native window.
    let declared = Rect::new(10.0, 20.0, 640.0, 480.0);

    assert!(window_bounds_need_update(None, declared));
    assert!(!window_bounds_need_update(Some(declared), declared));
    assert!(window_bounds_need_update(
        Some(Rect::new(10.0, 20.0, 800.0, 600.0)),
        declared
    ));
}

#[test]
fn colors_match_within_conversion_tolerance() {
    // Verifies tiny sRGB conversion drift does not rewrite the color panel.
    let color = Color::rgba(0.2, 0.4, 0.6, 1.0);
    assert!(colors_match(color, Color::rgba(0.2001, 0.4, 0.6, 1.0)));
    assert!(!colors_match(color, Color::rgba(0.3, 0.4, 0.6, 1.0)));
}
