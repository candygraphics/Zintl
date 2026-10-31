/// Implements `View` for wrappers through their native element.
macro_rules! native_view {
    ($($view:ident),* $(,)?) => {$(
        impl View for $view {
            type Output = RenderNode;

            fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
                self.native_element(cx)
            }
        }
    )*};
}

mod controls;

pub use controls::{
    ColorWell, ComboBox, NumberField, Picker, PickerStyle, ScrollView, SearchField, Slider, Tab,
    TabView, Toggle, ToggleStyle,
};
use zintl_ui::composer::Composer;
pub use zintl_ui::element::{Element, ElementFactory, IntoElement};
#[cfg(test)]
use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
pub use zintl_ui::store::Store;
pub use zintl_ui::view::View;
pub use zintl_ui_layout::{
    Axis, ChildSizing, CrossAxisAlignment, LayoutDimension, LayoutStyle, MainAxisDistribution, Size,
};
pub use zintl_ui_native::graphics::{
    FillRule, Font, FontWeight, GradientStop, Graphics2D, Image, LineCap, LineJoin, Paint, Path,
    Point, StrokeStyle, TextMetrics, Transform,
};
pub use zintl_ui_native::{
    Alert, AlertButton, AlertStyle, Attach, Attachments, ButtonRole, Canvas2D, Canvas2DHandler,
    CanvasEvent, CanvasEventContext, CanvasSetup, Color, ColorPanel, Context, EnglishMenuLocalizer,
    Event, EventKind, FileDialog, Frame, Invalidator, JapaneseMenuLocalizer, Key, KeyEvent,
    KeyPhase, ListFactory, MainMenu, Menu, MenuItem, MenuLocalizer, Modifier, Modifiers, NamedKey,
    NativeView, PointerButton, PointerEvent, PointerPhase, Popover, PopoverEdge, Rect, Redraw,
    RedrawPolicy, RenderNode, ScrollAxes, ScrollEvent, ScrollStatus, Shortcut, Sidebar,
    SidebarItem, SidebarSection, StandardAction, StandardMenu, StandardText, TitleBar, ViewKind,
    graphics, list,
};

native_view!(Text, Button, TextField, HStack, VStack);

#[derive(Clone)]
pub struct Window {
    inner: zintl_ui_native::Window,
}

impl Window {
    pub fn new(bounds: Rect, title: impl Into<String>) -> Self {
        Self {
            inner: zintl_ui_native::Window::new(bounds, title),
        }
    }

    /// Creates a floating utility panel, such as an inspector or palette.
    pub fn panel(bounds: Rect, title: impl Into<String>) -> Self {
        Self {
            inner: zintl_ui_native::Window::new(bounds, title)
                .style(zintl_ui_native::WindowStyle::Panel),
        }
    }

    /// Presents `alert` as a sheet while its bound Store is `true`.
    pub fn alert(mut self, alert: Alert) -> Self {
        self.inner = self.inner.alert(alert);
        self
    }

    /// Presents `dialog` as a sheet while its bound Store is `true`.
    pub fn file_dialog(mut self, dialog: FileDialog) -> Self {
        self.inner = self.inner.file_dialog(dialog);
        self
    }

    /// Shows the shared color panel while its bound Store is `true`.
    pub fn color_panel(mut self, panel: ColorPanel) -> Self {
        self.inner = self.inner.color_panel(panel);
        self
    }

    /// Shows the window while `open` is `true`; closing the window writes `false`.
    pub fn bind_open(mut self, open: Store<bool>) -> Self {
        self.inner = self.inner.bind_open(open);
        self
    }

    /// Runs after the user closes the window.
    pub fn on_close(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.inner = self.inner.on_close(action);
        self
    }

    pub fn sidebar(mut self, sidebar: Sidebar) -> Self {
        self.inner = self.inner.sidebar(sidebar);
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.inner = self.inner.id(id);
        self
    }

    /// Extends the content under the title bar or hides it, leaving the window buttons.
    pub fn title_bar(mut self, title_bar: TitleBar) -> Self {
        self.inner = self.inner.title_bar(title_bar);
        self
    }

    pub fn content<V>(self, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Window {
            inner: self.inner.content(content),
        }
    }
}

impl View for Window {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.inner.clone().into_element()
    }
}

#[derive(Clone)]
pub struct Text {
    inner: zintl_ui_native::TextField,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            inner: zintl_ui_native::TextField::label_with_string(content),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.inner = self.inner.id(id);
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.inner = self.inner.minimum_size(size);
        self
    }
}

impl NativeView for Text {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        self.inner.base_element(cx)
    }

    fn attachments(&self) -> &Attachments {
        self.inner.attachments()
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        self.inner.attachments_mut()
    }
}

#[derive(Clone)]
pub struct Button {
    inner: zintl_ui_native::Button,
}

impl Button {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            inner: zintl_ui_native::Button::new(title),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.inner = self.inner.id(id);
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.inner = self.inner.minimum_size(size);
        self
    }

    pub fn on_click(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.inner = self.inner.on_click(action);
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        let bezel = match style {
            ButtonStyle::Push => Some(zintl_ui_native::BezelStyle::Push),
            ButtonStyle::Flexible => Some(zintl_ui_native::BezelStyle::FlexiblePush),
            ButtonStyle::Borderless => None,
            ButtonStyle::Toolbar => Some(zintl_ui_native::BezelStyle::Toolbar),
            ButtonStyle::Accessory => Some(zintl_ui_native::BezelStyle::AccessoryBarAction),
            ButtonStyle::Circular => Some(zintl_ui_native::BezelStyle::Circular),
            ButtonStyle::Help => Some(zintl_ui_native::BezelStyle::Help),
            ButtonStyle::Disclosure => Some(zintl_ui_native::BezelStyle::PushDisclosure),
        };
        self.inner = self.inner.bezel_style(bezel);
        self
    }

    /// `Default` answers Return, `Cancel` answers Escape, and `Destructive` warns.
    pub fn role(mut self, role: ButtonRole) -> Self {
        self.inner = self.inner.role(role);
        self
    }

    /// Shows an SF Symbol; an empty title shows the symbol alone.
    pub fn system_image(mut self, name: impl Into<String>) -> Self {
        self.inner = self.inner.system_image(name);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.inner = self.inner.enabled(enabled);
        self
    }
}

/// The bezel drawn around a [`Button`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonStyle {
    /// The standard rounded push button.
    #[default]
    Push,
    /// A push button that grows to fit multi-line titles.
    Flexible,
    /// Only the title or symbol.
    Borderless,
    Toolbar,
    /// A compact button for accessory bars.
    Accessory,
    Circular,
    /// The round question mark button.
    Help,
    /// A disclosure triangle in a push button.
    Disclosure,
}

impl NativeView for Button {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        self.inner.base_element(cx)
    }

    fn attachments(&self) -> &Attachments {
        self.inner.attachments()
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        self.inner.attachments_mut()
    }
}

#[derive(Clone)]
pub struct TextField {
    inner: zintl_ui_native::TextField,
}

impl TextField {
    pub fn new() -> Self {
        Self {
            inner: zintl_ui_native::TextField::new(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.inner = self.inner.id(id);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.inner = self.inner.placeholder(placeholder);
        self
    }

    pub fn bind(mut self, store: Store<String>) -> Self {
        self.inner = self.inner.bind(store);
        self
    }

    pub fn multiline(mut self) -> Self {
        self.inner = self.inner.multiline();
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.inner = self.inner.minimum_size(size);
        self
    }
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for TextField {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        self.inner.base_element(cx)
    }

    fn attachments(&self) -> &Attachments {
        self.inner.attachments()
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        self.inner.attachments_mut()
    }
}

/// Shared state for HStack and VStack.
#[derive(Clone)]
struct Stack {
    children: ListFactory<RenderNode>,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl Stack {
    fn new(axis: Axis, children: ListFactory<RenderNode>) -> Self {
        Self {
            children,
            layout: LayoutStyle::stack(axis, 8.0),
            id: None,
            attachments: Attachments::default(),
        }
    }

    fn native_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let view = zintl_ui_native::View::new(self.layout, self.children.clone());
        match &self.id {
            Some(id) => view.id(id.clone()).native_element(cx),
            None => view.native_element(cx),
        }
    }
}

#[derive(Clone)]
pub struct HStack {
    stack: Stack,
}

impl HStack {
    pub fn new(children: ListFactory<RenderNode>) -> Self {
        Self {
            stack: Stack::new(Axis::Horizontal, children),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.stack.id = Some(id.into());
        self
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.stack.layout.gap = spacing;
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.stack.layout.width = LayoutDimension::Percent(1.0);
        self
    }

    pub fn space_between(mut self) -> Self {
        self.stack.layout.main_axis_distribution = MainAxisDistribution::SpaceBetween;
        self
    }

    pub fn equal_width_children(mut self) -> Self {
        self.stack.layout.child_sizing = ChildSizing::Equal;
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.stack.layout.minimum_size = size;
        self
    }

    /// Insets the children from every edge.
    pub fn padding(mut self, padding: f32) -> Self {
        self.stack.layout.padding = padding;
        self
    }

    /// Takes the remaining space along the parent's main axis.
    pub fn grow(mut self) -> Self {
        self.stack.layout.grow = 1.0;
        self
    }
}

impl NativeView for HStack {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        self.stack.native_element(cx)
    }

    fn attachments(&self) -> &Attachments {
        &self.stack.attachments
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        &mut self.stack.attachments
    }
}

#[derive(Clone)]
pub struct VStack {
    stack: Stack,
}

impl VStack {
    pub fn new(children: ListFactory<RenderNode>) -> Self {
        Self {
            stack: Stack::new(Axis::Vertical, children),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.stack.id = Some(id.into());
        self
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.stack.layout.gap = spacing;
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.stack.layout.width = LayoutDimension::Percent(1.0);
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.stack.layout.minimum_size = size;
        self
    }

    /// Insets the children from every edge.
    pub fn padding(mut self, padding: f32) -> Self {
        self.stack.layout.padding = padding;
        self
    }

    /// Takes the remaining space along the parent's main axis.
    pub fn grow(mut self) -> Self {
        self.stack.layout.grow = 1.0;
        self
    }
}

impl NativeView for VStack {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        self.stack.native_element(cx)
    }

    fn attachments(&self) -> &Attachments {
        &self.stack.attachments
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        &mut self.stack.attachments
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedNode {
    pub value: RenderNode,
    pub children: Vec<RenderedNode>,
}

// Tests use the headless backend everywhere so they behave alike on every platform.
#[cfg(all(target_os = "macos", not(test)))]
type DesktopBackend = zintl_ui_appkit_backend::AppKitBackend;

#[cfg(all(target_os = "windows", not(test)))]
type DesktopBackend = zintl_ui_winui3_backend::WinUI3Backend;

#[cfg(any(test, not(any(target_os = "macos", target_os = "windows"))))]
type DesktopBackend = zintl_ui_backend_core::HeadlessBackend;

type DesktopNodeId = <DesktopBackend as RenderBackend<RenderNode>>::NodeId;

pub struct App {
    composer: Composer<RenderNode, DesktopBackend>,
}

impl App {
    pub fn new<E>(root: E) -> Self
    where
        E: IntoElement<Output = RenderNode>,
    {
        #[allow(unused_mut)]
        let mut backend = DesktopBackend::new();
        // Native backends install the standard menus until the app declares its own.
        #[cfg(all(any(target_os = "macos", target_os = "windows"), not(test)))]
        backend.set_default_main_menu(MainMenu::standard().commands());
        let mut composer = Composer::new(backend);
        composer.mount(root);
        Self { composer }
    }

    pub fn render(&self) -> RenderNode {
        self.render_tree().value
    }

    pub fn render_tree(&self) -> RenderedNode {
        Self::rendered_node(self.composer.backend(), self.root_element())
    }

    fn root_element(&self) -> DesktopNodeId {
        let backend = self.composer.backend();
        *backend
            .children(backend.root())
            .first()
            .expect("the rendered tree must contain a root element")
    }

    fn rendered_node(backend: &DesktopBackend, id: DesktopNodeId) -> RenderedNode {
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

    #[cfg(all(target_os = "macos", not(test)))]
    pub fn run(self) -> Result<(), AppError> {
        zintl_ui_appkit_backend::run_composer(self.composer)
    }

    #[cfg(all(target_os = "windows", not(test)))]
    pub fn run(self) -> Result<(), AppError> {
        zintl_ui_winui3_backend::run_composer(self.composer)
    }

    #[cfg(test)]
    fn update_text_store(&mut self, store: Store<String>, value: String) {
        self.composer
            .context(|cx| cx.update(store, |current| *current = value));
        self.composer.flush();
    }

    #[cfg(test)]
    fn dispatch_root_event(&mut self, event: Event) -> bool {
        let route = self
            .composer
            .backend()
            .event_route(self.root_element())
            .expect("the rendered root must have an event route");
        self.composer.dispatch_event(route, event)
    }

    /// Dispatches to the node at `path`, starting with the root's children.
    #[cfg(test)]
    fn dispatch_at(&mut self, path: &[usize], event: Event) -> bool {
        let route = Self::event_route_at(self.composer.backend(), path)
            .expect("the addressed node must have an event route");
        self.composer.dispatch_event(route, event)
    }

    /// Renders every root-level element, such as a main menu and windows.
    #[cfg(test)]
    fn render_roots(&self) -> Vec<RenderedNode> {
        let backend = self.composer.backend();
        backend
            .children(backend.root())
            .iter()
            .map(|child| Self::rendered_node(backend, *child))
            .collect()
    }

    #[cfg(test)]
    fn event_route_at(backend: &DesktopBackend, path: &[usize]) -> Option<EventRouteId> {
        let mut node = backend.root();
        for index in path {
            node = *backend.children(node).get(*index)?;
        }
        backend.event_route(node)
    }
}

#[cfg(target_os = "macos")]
pub use zintl_ui_appkit_backend::AppError;
#[cfg(target_os = "windows")]
pub use zintl_ui_winui3_backend::AppError;

#[cfg(test)]
mod widget_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

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
    fn desktop_controls_and_stacks_are_views() {
        // Verifies every desktop primitive participates in the View abstraction.
        assert_view::<Text>();
        assert_view::<Button>();
        assert_view::<TextField>();
        assert_view::<HStack>();
        assert_view::<VStack>();
    }

    #[test]
    fn text_and_text_field_share_the_native_render_kind() {
        // Verifies desktop text wrappers both render through text field configuration.
        let label = App::new(Text::new("Label")).render();
        let field = App::new(TextField::new()).render();

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
    fn text_field_multiline_is_opt_in() {
        // Verifies the desktop wrapper keeps one line by default and forwards multiline mode.
        let single_line = App::new(TextField::new()).render();
        let multiline = App::new(TextField::new().multiline()).render();

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
    fn window_element_preserves_bounds_and_title() {
        // Verifies the Window wrapper preserves its declarative bounds and title.
        let bounds = Rect::new(10.0, 20.0, 640.0, 480.0);
        let app = App::new(Window::new(bounds, "Zintl"));
        assert_eq!(
            app.render(),
            RenderNode::Window {
                sidebar: None,
                bounds,
                title: "Zintl".into(),
                title_bar: TitleBar::Standard,
                id: None,
                style: zintl_ui_native::WindowStyle::Normal,
            }
        );
    }

    #[test]
    fn window_title_bar_reaches_the_render_node() {
        // Verifies the declared title bar style is carried to the backend.
        let bounds = Rect::new(10.0, 20.0, 640.0, 480.0);
        let app = App::new(Window::new(bounds, "Zintl").title_bar(TitleBar::Overlay));

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
        // Verifies stack wrappers preserve layout policy while using native render nodes.
        let app = App::new(
            Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Zintl")
                .content(HStack::new(list![Button::new("Save"), TextField::new()]).spacing(12.0)),
        );
        let tree = app.render_tree();
        let stack = &tree.children[0];

        assert_eq!(
            stack.value,
            RenderNode::View {
                kind: ViewKind::Container,
                layout: LayoutStyle::stack(Axis::Horizontal, 12.0),
                id: None,
                enabled: true,
                title_bar_interactive: false,
            }
        );
        assert!(matches!(
            stack.children[0].value,
            RenderNode::View {
                kind: ViewKind::Button(zintl_ui_backend_core::Button { .. }),
                layout: LayoutStyle {
                    minimum_size: Size {
                        width: 80.0,
                        height: 32.0
                    },
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
                    minimum_size: Size {
                        width: 160.0,
                        height: 28.0
                    },
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn stack_list_children_have_no_arity_limit() {
        // Verifies desktop stacks render heterogeneous lists longer than six children.
        let app = App::new(VStack::new(list![
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
        // Verifies desktop stack options reach the wrapped container layout.
        let app = App::new(
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
        let mut app = App::new(StoreTextFieldView {
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

        let stored = app.composer.context(|cx| cx.get(store).clone());
        assert_eq!(stored, "typed value");
        assert!(matches!(
            app.render(),
            RenderNode::View { kind: ViewKind::TextField { value, .. }, .. } if value == "typed value"
        ));
    }

    #[test]
    fn button_dispatches_its_registered_action() {
        // Verifies the desktop Button delegates activation to the native view route.
        let activations = Rc::new(Cell::new(0));
        let received = activations.clone();
        let mut app = App::new(Button::new("Save").on_click(move |_| {
            received.set(received.get() + 1);
        }));

        assert!(app.dispatch_root_event(Event::Activated));
        assert_eq!(activations.get(), 1);
    }

    #[test]
    fn view_ids_are_preserved_in_the_render_tree() {
        // Verifies wrapper IDs reach native render nodes for accessibility.
        let app = App::new(
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
        // Verifies wrappers preserve fine-grained Store watcher rebuilding.
        let renders = Rc::new(Cell::new(0));
        let captured = Rc::new(Cell::new(None));
        let mut app = App::new(BoundTextView {
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

        app.update_text_store(store, "updated".into());

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

    #[test]
    fn sidebar_selection_round_trips_and_rejects_unknown_items() {
        // Verifies the desktop Sidebar preserves Store selection behavior.
        struct Navigation {
            selection: Store<Option<String>>,
            captured: Rc<Cell<Store<Option<String>>>>,
        }

        impl View for Navigation {
            type Output = RenderNode;

            fn init(&mut self, cx: &mut Context<'_>) {
                self.selection = cx.store(Some("home".into()));
                self.captured.set(self.selection);
            }

            fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
                Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Navigation").sidebar(
                    Sidebar::new([SidebarSection::new([
                        SidebarItem::new("home", "Home"),
                        SidebarItem::new("settings", "Settings"),
                    ])])
                    .bind(self.selection),
                )
            }
        }

        let captured = Rc::new(Cell::new(Store::default()));
        let mut app = App::new(Navigation {
            selection: Store::default(),
            captured: captured.clone(),
        });
        assert!(app.dispatch_root_event(Event::SidebarSelectionChanged {
            id: "settings".into(),
        }));
        assert_eq!(
            app.composer.context(|cx| cx.get(captured.get()).clone()),
            Some("settings".into())
        );
        assert!(app.dispatch_root_event(Event::SidebarSelectionChanged {
            id: "missing".into(),
        }));
        assert_eq!(
            app.composer.context(|cx| cx.get(captured.get()).clone()),
            Some("settings".into())
        );
    }
}
