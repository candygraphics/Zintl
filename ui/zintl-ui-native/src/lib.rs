//! Declarative native desktop views that render core `RenderNode` descriptions.

mod attach;
mod button;
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
    AlertStyle, BezelStyle, ButtonType, Color, Event, EventKind, NodeId, PopoverEdge, Rect,
    RenderNode, ScrollAxes, ViewKind, WindowStyle,
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
    fn window_full_size_content_view_is_opt_in() {
        // Verifies Window::new uses normal content unless the builder enables full-size content.
        let normal = composer(Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Normal"));
        let full_size = composer(
            Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Full Size").full_size_content_view(),
        );
        let normal = normal.backend().value(root_id(&normal)).unwrap();
        let full_size = full_size.backend().value(root_id(&full_size)).unwrap();

        assert!(matches!(
            normal,
            RenderNode::Window {
                full_size_content_view: false,
                ..
            }
        ));
        assert!(matches!(
            full_size,
            RenderNode::Window {
                full_size_content_view: true,
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
