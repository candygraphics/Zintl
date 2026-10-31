//! Declarative native desktop views that render core `RenderNode` descriptions.

mod attach;
mod button;
mod canvas;
mod containers;
mod controls;
mod menu;
mod presentation;
mod sidebar;
mod text_field;
mod types;
mod view;
mod window;

pub use attach::{Attach, Attachments, NativeView};
pub use button::Button;
pub use canvas::Canvas2D;
pub use containers::{ScrollView, TabView, TabViewItem};
pub use controls::{
    ColorWell, ComboBox, NumberField, PopUpButton, SearchField, SegmentedControl, Slider, Switch,
};
pub use menu::{
    EnglishMenuLocalizer, JapaneseMenuLocalizer, MainMenu, Menu, MenuItem, MenuItemState,
    MenuLocalizer, MenuRole, MenuState, Modifier, Shortcut, StandardAction, StandardMenu,
    StandardText,
};
pub use presentation::{Alert, AlertButton, ColorPanel, FileDialog, Popover};
pub use sidebar::{Sidebar, SidebarItem, SidebarSection};
pub use text_field::TextField;
pub use types::{ButtonRole, ScrollStatus};
pub use view::View;
pub use window::Window;
pub use zintl_ui::element::{Element, IntoElement, ListFactory};
pub use zintl_ui::list;
pub use zintl_ui::store::Store;
pub use zintl_ui_backend_core::{
    AlertStyle, BezelStyle, ButtonType, Canvas2DHandler, CanvasEvent, CanvasEventContext,
    CanvasNode, CanvasSetup, Color, Event, EventKind, Frame, Invalidator, Key, KeyEvent, KeyPhase,
    Modifiers, NamedKey, NodeId, PointerButton, PointerEvent, PointerPhase, PopoverEdge, Rect,
    Redraw, RedrawPolicy, RenderNode, ScrollAxes, ScrollEvent, TitleBar, ViewKind, WindowStyle,
    graphics,
};
pub use zintl_ui_layout::{LayoutStyle, Size};

/// The context views receive while rendering.
pub type Context<'a> = <RenderNode as zintl_ui::renderer::RenderNode>::Context<'a>;

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use zintl_ui::composer::Composer;
    use zintl_ui::renderer::{RenderBackend, RenderNode as _};
    use zintl_ui::view::View as ViewTrait;
    use zintl_ui_backend_core::{HeadlessBackend, ViewKind};
    use zintl_ui_layout::{Axis, LayoutStyle};

    use super::*;

    fn composer<E>(root: E) -> Composer<RenderNode, HeadlessBackend>
    where
        E: IntoElement<Output = RenderNode>,
    {
        let mut composer = Composer::new(HeadlessBackend::new());
        composer.mount(root);
        composer
    }

    fn root_id(composer: &Composer<RenderNode, HeadlessBackend>) -> NodeId {
        let backend = composer.backend();
        *backend
            .children(backend.root())
            .first()
            .expect("the test tree must contain a root")
    }

    fn assert_view<T: ViewTrait<Output = RenderNode>>() {}

    #[test]
    fn native_widgets_are_zintl_views() {
        // Verifies each native widget participates in declarative composition.
        assert_view::<Button>();
        assert_view::<TextField>();
        assert_view::<View>();
        assert_view::<Window>();
    }

    #[test]
    fn list_children_preserve_mixed_view_order() {
        // Verifies containers accept ordered heterogeneous list children.
        let composer = composer(View::new(
            LayoutStyle::stack(Axis::Vertical, 8.0),
            list![Button::new("First"), TextField::new(), Button::new("Last")],
        ));
        let children = composer.backend().children(root_id(&composer));

        assert_eq!(children.len(), 3);
        assert!(matches!(
            composer.backend().value(children[0]),
            Some(RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { title, .. }), .. }) if title == "First"
        ));
        assert!(matches!(
            composer.backend().value(children[1]),
            Some(RenderNode::View {
                kind: ViewKind::TextField { .. },
                ..
            })
        ));
        assert!(matches!(
            composer.backend().value(children[2]),
            Some(RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { title, .. }), .. }) if title == "Last"
        ));
    }

    #[test]
    fn text_field_presentations_share_one_native_kind() {
        // Verifies labels and editable fields are configurations of one text field node.
        let label = composer(TextField::label_with_string("Label"));
        let field = composer(TextField::new());
        let label = label.backend().value(root_id(&label)).unwrap();
        let field = field.backend().value(root_id(&field)).unwrap();

        assert!(matches!(
            label,
            RenderNode::View {
                kind: ViewKind::TextField {
                    editable: false,
                    selectable: false,
                    bordered: false,
                    draws_background: false,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            field,
            RenderNode::View {
                kind: ViewKind::TextField {
                    editable: true,
                    selectable: true,
                    bordered: true,
                    draws_background: true,
                    ..
                },
                ..
            }
        ));
        assert!(label.same_kind(field));
    }

    #[test]
    fn text_field_multiline_is_opt_in() {
        // Verifies text fields are single-line by default and opt into multiline rendering.
        let single_line = composer(TextField::new());
        let multiline = composer(TextField::new().multiline());
        let single_line = single_line.backend().value(root_id(&single_line)).unwrap();
        let multiline = multiline.backend().value(root_id(&multiline)).unwrap();

        assert!(matches!(
            single_line,
            RenderNode::View {
                kind: ViewKind::TextField {
                    multiline: false,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            multiline,
            RenderNode::View {
                kind: ViewKind::TextField {
                    multiline: true,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn window_title_bar_is_standard_by_default() {
        // Verifies Window::new keeps the standard title bar unless the builder changes it.
        let normal = composer(Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Normal"));
        let full_size = composer(
            Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Hidden").title_bar(TitleBar::Hidden),
        );
        let normal = normal.backend().value(root_id(&normal)).unwrap();
        let full_size = full_size.backend().value(root_id(&full_size)).unwrap();

        assert!(matches!(
            normal,
            RenderNode::Window {
                title_bar: TitleBar::Standard,
                ..
            }
        ));
        assert!(matches!(
            full_size,
            RenderNode::Window {
                title_bar: TitleBar::Hidden,
                ..
            }
        ));
    }

    #[test]
    fn text_binding_and_button_action_route_through_views() {
        // Verifies controls connect native events to Store updates and actions.
        let mut composer = Composer::new(HeadlessBackend::new());
        let value = composer.context(|cx: &mut Context<'_>| cx.store(String::new()));
        let changes = Rc::new(Cell::new(0));
        let received = changes.clone();
        composer.mount(View::new(
            LayoutStyle::stack(Axis::Vertical, 8.0),
            list![
                TextField::new().bind(value),
                Button::new("Save").on_click(move |_| received.set(received.get() + 1)),
            ],
        ));
        let root = root_id(&composer);
        let field = composer.backend().children(root)[0];
        let button = composer.backend().children(root)[1];
        let field_route = composer.backend().event_route(field).unwrap();
        let button_route = composer.backend().event_route(button).unwrap();

        assert!(composer.dispatch_event(
            field_route,
            Event::TextChanged {
                value: "typed".into(),
            },
        ));
        assert!(composer.dispatch_event(button_route, Event::Activated));
        assert_eq!(composer.context(|cx| cx.get(value).clone()), "typed");
        assert_eq!(changes.get(), 1);
    }

    /// Draws a bar `width` points wide and counts its draws.
    #[derive(Clone)]
    struct Bar {
        width: f64,
        draws: Rc<Cell<u32>>,
    }

    impl Canvas2DHandler for Bar {
        fn draw(&mut self, g: &mut graphics::Graphics2D<'_>, _frame: &Frame) {
            self.draws.set(self.draws.get() + 1);
            g.fill_rect(Rect::new(0.0, 0.0, self.width, 1.0), Color::BLACK);
        }

        fn reconcile(&mut self, next: Self) -> Redraw {
            // Keep the mounted counter so the test sees draws of one handler.
            self.width = next.width;
            Redraw::Yes
        }
    }

    fn canvas(node: &RenderNode) -> CanvasNode {
        match node {
            RenderNode::View {
                kind: ViewKind::Canvas2D { canvas, .. },
                ..
            } => canvas.clone(),
            _ => panic!("expected a canvas node"),
        }
    }

    fn frame() -> Frame {
        Frame {
            bounds: Rect::new(0.0, 0.0, 10.0, 10.0),
            scale: 1.0,
            time: 0.0,
            delta: 0.0,
        }
    }

    #[test]
    fn canvas_rerender_reconciles_into_the_mounted_handler() {
        // Verifies a Store update keeps the mounted canvas and hands it the new handler.
        let mut composer = Composer::new(HeadlessBackend::new());
        let width = composer.context(|cx: &mut Context<'_>| cx.store(2.0));
        let draws = Rc::new(Cell::new(0));
        let counter = draws.clone();
        let root = composer.context(|cx| {
            cx.watch(width, move |width| {
                Canvas2D::new(Bar {
                    width: *width,
                    draws: counter.clone(),
                })
            })
        });
        composer.mount(root);
        let node = root_id(&composer);
        let mounted = canvas(composer.backend().value(node).unwrap());
        mounted.record(&frame());

        composer.context(|cx| cx.update(width, |width| *width = 5.0));
        composer.flush();
        let current = canvas(composer.backend().value(root_id(&composer)).unwrap());

        assert_eq!(current, mounted);
        let commands = current.record(&frame());
        assert!(
            matches!(&commands[..], [graphics::DrawCommand::Fill { path, .. }]
            if *path == graphics::Path::rect(Rect::new(0.0, 0.0, 5.0, 1.0)))
        );
        assert_eq!(draws.get(), 2);
    }

    #[test]
    fn canvas_events_route_to_on_event() {
        // Verifies canvas input reaches the declared action with a Context.
        let mut composer = Composer::new(HeadlessBackend::new());
        let clicks = composer.context(|cx: &mut Context<'_>| cx.store(0));
        composer.mount(
            Canvas2D::new(Bar {
                width: 1.0,
                draws: Rc::default(),
            })
            .on_event(move |cx, event| {
                if matches!(
                    event,
                    CanvasEvent::Pointer(PointerEvent {
                        phase: PointerPhase::Down,
                        ..
                    })
                ) {
                    cx.update(clicks, |clicks| *clicks += 1);
                }
            }),
        );
        let route = composer.backend().event_route(root_id(&composer)).unwrap();
        let down = CanvasEvent::Pointer(PointerEvent {
            phase: PointerPhase::Down,
            button: Some(PointerButton::Primary),
            location: graphics::Point::new(1.0, 1.0),
            modifiers: Modifiers::default(),
            click_count: 1,
        });

        assert!(composer.dispatch_event(route, Event::Canvas(down)));
        assert_eq!(composer.context(|cx| *cx.get(clicks)), 1);
    }

    #[test]
    fn title_bar_interactive_is_opt_in_per_view() {
        // Verifies only views marked with the modifier keep input under the title bar.
        let plain = composer(Button::new("Plain"));
        let marked = composer(Button::new("Marked").title_bar_interactive());
        let plain = plain.backend().value(root_id(&plain)).unwrap();
        let marked = marked.backend().value(root_id(&marked)).unwrap();

        assert!(matches!(
            plain,
            RenderNode::View {
                title_bar_interactive: false,
                ..
            }
        ));
        assert!(matches!(
            marked,
            RenderNode::View {
                title_bar_interactive: true,
                ..
            }
        ));
    }

    #[test]
    fn canvases_of_different_handlers_are_different_kinds() {
        // Verifies switching handler types recreates the canvas instead of reconciling.
        #[derive(Clone)]
        struct Empty;
        impl Canvas2DHandler for Empty {
            fn draw(&mut self, _g: &mut graphics::Graphics2D<'_>, _frame: &Frame) {}
        }
        let bar = composer(Canvas2D::new(Bar {
            width: 1.0,
            draws: Rc::default(),
        }));
        let empty = composer(Canvas2D::new(Empty));
        let bar = bar.backend().value(root_id(&bar)).unwrap();
        let empty = empty.backend().value(root_id(&empty)).unwrap();
        assert!(!bar.same_kind(empty));
    }

    #[test]
    fn sidebar_binding_filters_unknown_native_ids() {
        // Verifies sidebar selection updates only for declared items.
        let mut composer = Composer::new(HeadlessBackend::new());
        let selection = composer.context(|cx: &mut Context<'_>| cx.store(None::<String>));
        composer.mount(
            Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Navigation").sidebar(
                Sidebar::new([SidebarSection::new([SidebarItem::new("home", "Home")])])
                    .bind(selection),
            ),
        );
        let root = root_id(&composer);
        let route = composer.backend().event_route(root).unwrap();

        assert!(
            composer.dispatch_event(route, Event::SidebarSelectionChanged { id: "home".into() },)
        );
        assert_eq!(
            composer.context(|cx| cx.get(selection).clone()),
            Some("home".into())
        );
        assert!(composer.dispatch_event(
            route,
            Event::SidebarSelectionChanged {
                id: "missing".into(),
            },
        ));
        assert_eq!(
            composer.context(|cx| cx.get(selection).clone()),
            Some("home".into())
        );
    }
}
