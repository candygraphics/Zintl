//! Contracts for text, buttons, stacks, and windows.

use std::cell::Cell;
use std::rc::Rc;

use zintl_ui::view::View;
use zintl_ui_backend_core::WindowStyle;
use zintl_ui_layout::{
    Axis, ChildSizing, CrossAxisAlignment, LayoutDimension, LayoutStyle, MainAxisDistribution, Size,
};

use crate::graphics::Font;
use crate::testing::TestApp;
use crate::*;

struct StoreTextFieldView {
    value: Option<Store<String>>,
    captured: Rc<Cell<Option<Store<String>>>>,
}

struct BoundTextView {
    value: Option<Store<String>>,
    renders: Rc<Cell<usize>>,
    captured: Rc<Cell<Option<Store<String>>>>,
}

impl View for BoundTextView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.value = Some(cx.store("initial".to_owned()));
        self.captured.set(self.value);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        self.renders.set(self.renders.get() + 1);
        let store = self
            .value
            .expect("BoundTextView must be initialized before rendering");
        VStack::new(list![
            TextField::new().bind(store),
            cx.watch(store, |value| Text::new(format!("Stored value: {value}"))),
        ])
    }
}

impl View for StoreTextFieldView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.value = Some(cx.store("initial".to_owned()));
        self.captured.set(self.value);
    }

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        TextField::new().bind(
            self.value
                .expect("StoreTextFieldView must be initialized before rendering"),
        )
    }
}

fn assert_view<T: View<Output = RenderNode>>() {}

#[test]
fn text_buttons_and_stacks_are_views() {
    // Verifies text, buttons, and stacks participate in the View abstraction.
    assert_view::<Text>();
    assert_view::<Button>();
    assert_view::<TextField>();
    assert_view::<HStack>();
    assert_view::<VStack>();
    assert_view::<Spacer>();
}

#[test]
fn stack_sizes_and_alignment_reach_the_layout() {
    // Verifies fixed sizes and cross-axis alignment are carried on the stack's layout style.
    let stack = TestApp::new(
        VStack::new(list![])
            .width(240.0)
            .height(80.0)
            .alignment(CrossAxisAlignment::Start),
    )
    .render();

    let RenderNode::View { layout, .. } = stack else {
        panic!("expected a view, got {stack:?}");
    };
    assert_eq!(layout.width, LayoutDimension::Points(240.0));
    assert_eq!(layout.height, LayoutDimension::Points(80.0));
    assert_eq!(layout.cross_axis_alignment, CrossAxisAlignment::Start);
}

#[test]
fn spacer_is_an_empty_growing_container() {
    // Verifies a spacer takes free main-axis space without a minimum size.
    let spacer = TestApp::new(Spacer::new()).render();

    let RenderNode::View {
        kind: ViewKind::Container,
        layout,
        ..
    } = spacer
    else {
        panic!("expected a container, got {spacer:?}");
    };
    assert_eq!(layout.grow, 1.0);
    assert_eq!(layout.minimum_size, Some(Size::new(0.0, 0.0)));
}

#[test]
fn text_and_text_field_share_the_native_render_kind() {
    // Verifies text and text fields both render through text field configuration.
    let label = TestApp::new(Text::new("Label")).render();
    let field = TestApp::new(TextField::new()).render();

    assert!(matches!(
        label,
        RenderNode::View {
            kind: ViewKind::TextField {
                editable: false,
                bordered: false,
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
                bordered: true,
                ..
            },
            ..
        }
    ));
}

#[test]
fn text_style_reaches_the_render_node_and_sizes_the_label() {
    // Verifies text style is forwarded, and a line limit lets the label shrink to any width
    // while it still prefers its text's width.
    let text = TestApp::new(
        Text::new("Groceries")
            .font(Font::system(20.0).bold())
            .color(TextColor::Secondary)
            .alignment(TextAlignment::Center)
            .line_limit(2),
    )
    .render();

    let RenderNode::View {
        kind: ViewKind::TextField { style, .. },
        layout,
        ..
    } = text
    else {
        panic!("expected a text field, got {text:?}");
    };
    assert_eq!(style.font, Some(Font::system(20.0).bold()));
    assert_eq!(style.color, TextColor::Secondary);
    assert_eq!(style.alignment, TextAlignment::Center);
    assert_eq!(style.line_limit, Some(2));
    let minimum = layout.minimum_size.expect("labels declare a minimum");
    assert_eq!(minimum.width, 0.0);
    assert!(minimum.height >= 50.0);
    let content = layout
        .content_size
        .expect("line-limited labels prefer their text");
    assert!(content.width > 50.0);
    assert_eq!(content.height, minimum.height);
}

#[test]
fn window_element_preserves_bounds_and_title() {
    // Verifies a window preserves its declarative bounds and title.
    let bounds = Rect::new(10.0, 20.0, 640.0, 480.0);
    let app = TestApp::new(Window::new(bounds, "Zintl"));
    assert_eq!(
        app.render(),
        RenderNode::Window {
            sidebar: None,
            bounds,
            title: "Zintl".into(),
            title_bar: TitleBar::Standard,
            id: None,
            style: WindowStyle::Normal,
            toolbar: None,
            autosave: None,
            document: Default::default(),
        }
    );
}

#[test]
fn window_title_bar_reaches_the_render_node() {
    // Verifies the declared title bar style is carried to the backend.
    let bounds = Rect::new(10.0, 20.0, 640.0, 480.0);
    let app = TestApp::new(Window::new(bounds, "Zintl").title_bar(TitleBar::Overlay));

    assert!(matches!(
        app.render(),
        RenderNode::Window {
            title_bar: TitleBar::Overlay,
            ..
        }
    ));
}

#[test]
fn stack_render_tree_carries_layout_and_control_sizes() {
    // Verifies stacks preserve layout policy while using native render nodes.
    let app = TestApp::new(
        Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Zintl")
            .content(HStack::new(list![Button::new("Save"), TextField::new()]).spacing(12.0)),
    );
    let tree = app.render_tree();
    let stack = &tree.children[0];
    // Rows center their children by default.
    let mut row = LayoutStyle::stack(Axis::Horizontal, 12.0);
    row.cross_axis_alignment = CrossAxisAlignment::Center;

    assert_eq!(
        stack.value,
        RenderNode::View {
            kind: ViewKind::Container,
            layout: row,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        }
    );
    assert!(matches!(
        stack.children[0].value,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button { .. }),
            layout: LayoutStyle {
                minimum_size: Some(Size {
                    width: 80.0,
                    height: 32.0
                }),
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        stack.children[1].value,
        RenderNode::View {
            kind: ViewKind::TextField { .. },
            layout: LayoutStyle {
                minimum_size: Some(Size {
                    width: 160.0,
                    height: 28.0
                }),
                ..
            },
            ..
        }
    ));
}

#[test]
fn stack_list_children_have_no_arity_limit() {
    // Verifies stacks render heterogeneous lists longer than six children.
    let app = TestApp::new(VStack::new(list![
        Text::new("One"),
        Button::new("Two"),
        TextField::new(),
        Text::new("Four"),
        Button::new("Five"),
        TextField::new(),
        Text::new("Seven"),
        Button::new("Eight"),
    ]));
    let tree = app.render_tree();

    assert_eq!(tree.children.len(), 8);
    assert!(matches!(
        tree.children[0].value,
        RenderNode::View {
            kind: ViewKind::TextField {
                editable: false,
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        tree.children[1].value,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button { .. }),
            ..
        }
    ));
    assert!(matches!(
        tree.children[2].value,
        RenderNode::View {
            kind: ViewKind::TextField { editable: true, .. },
            ..
        }
    ));
}

#[test]
fn stack_builders_preserve_width_and_child_distribution() {
    // Verifies stack options reach the container layout.
    let app = TestApp::new(
        Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Zintl").content(
            VStack::new(list![
                HStack::new(list![Button::new("Save"), Button::new("Cancel")])
                    .fill_width()
                    .space_between()
                    .equal_width_children()
            ])
            .fill_width(),
        ),
    );
    let tree = app.render_tree();
    let column = &tree.children[0];
    let row = &column.children[0];

    assert!(matches!(
        column.value,
        RenderNode::View {
            kind: ViewKind::Container,
            layout: LayoutStyle {
                width: LayoutDimension::Percent(1.0),
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        row.value,
        RenderNode::View {
            kind: ViewKind::Container,
            layout: LayoutStyle {
                width: LayoutDimension::Percent(1.0),
                main_axis_distribution: MainAxisDistribution::SpaceBetween,
                child_sizing: ChildSizing::Equal,
                ..
            },
            ..
        }
    ));
}

#[test]
fn text_field_writes_native_input_to_its_store() {
    // Verifies a bound TextField reads native input into its Store and rerenders.
    let captured = Rc::new(Cell::new(None));
    let mut app = TestApp::new(StoreTextFieldView {
        value: None,
        captured: captured.clone(),
    });
    assert!(
        matches!(app.render(), RenderNode::View { kind: ViewKind::TextField { value, .. }, .. } if value == "initial")
    );
    let store = captured.get().expect("init must publish the test Store");

    assert!(app.dispatch_root_event(Event::TextChanged {
        value: "typed value".into(),
    }));

    let stored = app.get(store);
    assert_eq!(stored, "typed value");
    assert!(matches!(
        app.render(),
        RenderNode::View { kind: ViewKind::TextField { value, .. }, .. } if value == "typed value"
    ));
}

#[test]
fn button_dispatches_its_registered_action() {
    // Verifies a button runs its action when activated.
    let activations = Rc::new(Cell::new(0));
    let received = activations.clone();
    let mut app = TestApp::new(Button::new("Save").on_click(move |_| {
        received.set(received.get() + 1);
    }));

    assert!(app.dispatch_root_event(Event::Activated));
    assert_eq!(activations.get(), 1);
}

#[test]
fn view_ids_are_preserved_in_the_render_tree() {
    // Verifies IDs reach the render nodes for accessibility.
    let app = TestApp::new(
        Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Zintl")
            .id("main-window")
            .content(TextField::new().id("name-input")),
    );
    let tree = app.render_tree();

    assert!(matches!(
        &tree.value,
        RenderNode::Window { id: Some(id), .. } if id == "main-window"
    ));
    assert!(matches!(
        &tree.children[0].value,
        RenderNode::View { kind: ViewKind::TextField { .. }, id: Some(id), .. } if id == "name-input"
    ));
}

#[test]
fn store_watcher_rebuilds_only_its_dependent_element() {
    // Verifies views keep fine-grained Store watcher rebuilding.
    let renders = Rc::new(Cell::new(0));
    let captured = Rc::new(Cell::new(None));
    let mut app = TestApp::new(BoundTextView {
        value: None,
        renders: renders.clone(),
        captured: captured.clone(),
    });
    let tree = app.render_tree();
    assert!(matches!(
        &tree.children[0].value,
        RenderNode::View {
            kind: ViewKind::TextField { .. },
            ..
        }
    ));
    let store = captured.get().expect("init must publish the test Store");

    app.set(store, "updated".to_owned());

    let tree = app.render_tree();
    let RenderNode::View {
        kind: ViewKind::TextField { value, .. },
        ..
    } = &tree.children[1].value
    else {
        panic!("the watched Text must remain a text field")
    };
    assert_eq!(value, "Stored value: updated");
    assert_eq!(renders.get(), 1);
}
