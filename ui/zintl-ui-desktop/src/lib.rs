//! The desktop app runner and the views it shows.

use zintl_ui::composer::Composer;
pub use zintl_ui::element::{Element, ElementFactory, IntoElement, KeyedElement};
use zintl_ui::renderer::RenderBackend;
pub use zintl_ui::store::Store;
pub use zintl_ui::view::View;
pub use zintl_ui_layout::{
    Axis, ChildSizing, CrossAxisAlignment, LayoutDimension, LayoutStyle, MainAxisDistribution, Size,
};
pub use zintl_ui_native::graphics::{
    AttributedText, FillRule, Font, FontWeight, GradientStop, Graphics2D, HorizontalAlign, Image,
    LineCap, LineJoin, Paint, ParagraphStyle, Path, Point, SpanStyle, StrokeStyle, TextAlign,
    TextMetrics, Transform, VerticalAlign,
};
pub use zintl_ui_native::{
    AccessibilityRole, Alert, AlertButton, AlertStyle, AnnouncementPriority, Attach, BarChart,
    Button, ButtonRole, ButtonStyle, CalendarDate, Canvas2D, Canvas2DHandler, CanvasEvent,
    CanvasEventContext, CanvasId, CanvasSetup, ChartBar, Color, ColorPanel, ColorWell, ComboBox,
    Context, Control, Cursor, DatePicker, DatePickerStyle, DisclosureGroup, EnglishMenuLocalizer,
    Event, EventKind, FileDialog, Frame, GesturePhase, Grid, GridCell, HStack, IntegerField,
    Invalidator, JapaneseMenuLocalizer, Key, KeyEvent, KeyPhase, Layout, LayoutOverrides, List,
    ListFactory, ListRow, ListSection, ListStyle, MagnifyEvent, MainMenu, Menu, MenuItem,
    MenuLocalizer, Modifier, Modifiers, NamedKey, NativeView, NumberField, OptionalDatePicker,
    Palette, PaletteColor, Picker, PickerStyle, PointerButton, PointerEvent, PointerPhase, Popover,
    PopoverEdge, ProgressIndicator, Rect, Redraw, RedrawPolicy, RenderNode, ScrollAlignment,
    ScrollAxes, ScrollEvent, ScrollStatus, ScrollView, SearchField, Separator, Shortcut,
    ShortcutKey, Sidebar, SidebarItem, SidebarSection, Signal, Slider, SortOrder, Spacer,
    SplitView, StandardAction, StandardMenu, StandardText, StoreSender, SystemColor, Tab, TabView,
    Table, TableColumn, TableRow, Text, TextAlignment, TextColor, TextEditor, TextField, TitleBar,
    Toggle, ToggleStyle, Toolbar, ToolbarItem, UndoHandler, VStack, ViewKind, ViewModifiers,
    Window, ZStack, graphics, list,
};
#[cfg(feature = "wgpu")]
pub use zintl_ui_native::{
    WgpuCanvas, WgpuCanvasHandler, WgpuContext, WgpuError, WgpuSetup, WgpuTarget, wgpu,
};

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
    on_exit: Option<Box<dyn FnOnce()>>,
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
        Self {
            composer,
            on_exit: None,
        }
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

    /// Runs once before the process ends, whether the last window closed or
    /// the user quit, for example to finish saving.
    ///
    /// On macOS, ⌘Q exits the process without returning from [`App::run`],
    /// so work after `run` is not enough.
    pub fn on_exit(mut self, action: impl FnOnce() + 'static) -> Self {
        self.on_exit = Some(Box::new(action));
        self
    }

    #[cfg(all(target_os = "macos", not(test)))]
    pub fn run(self) -> Result<(), AppError> {
        let on_exit = self.on_exit.unwrap_or_else(|| Box::new(|| {}));
        zintl_ui_appkit_backend::run_composer(self.composer, on_exit)
    }

    #[cfg(all(target_os = "windows", not(test)))]
    pub fn run(self) -> Result<(), AppError> {
        let on_exit = self.on_exit.unwrap_or_else(|| Box::new(|| {}));
        zintl_ui_winui3_backend::run_composer(self.composer, on_exit)
    }
}

#[cfg(target_os = "macos")]
pub use zintl_ui_appkit_backend::AppError;
#[cfg(target_os = "windows")]
pub use zintl_ui_winui3_backend::AppError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_renders_its_root_and_the_nodes_below_it() {
        // Verifies an app mounts its root view and reads the tree back as render nodes.
        let bounds = Rect::new(10.0, 20.0, 640.0, 480.0);
        let app =
            App::new(Window::new(bounds, "Zintl").content(VStack::new(list![Text::new("Hello")])));
        let tree = app.render_tree();

        assert!(matches!(
            app.render(),
            RenderNode::Window { bounds: window, title, .. } if window == bounds && title == "Zintl"
        ));
        assert_eq!(tree.value, app.render());
        assert!(matches!(
            tree.children[0].value,
            RenderNode::View {
                kind: ViewKind::Container,
                ..
            }
        ));
        assert_eq!(tree.children[0].children.len(), 1);
    }

    #[test]
    fn views_are_reachable_from_the_desktop_crate() {
        // Verifies every view stays public here, so apps depend on this crate alone.
        #[allow(unused_imports)]
        use crate::{
            Alert, BarChart, Button, ButtonStyle, Canvas2D, ChartBar, ColorPanel, ColorWell,
            ComboBox, DatePicker, DisclosureGroup, FileDialog, Grid, GridCell, HStack,
            IntegerField, List, MainMenu, NumberField, OptionalDatePicker, Picker, PickerStyle,
            Popover, ProgressIndicator, ScrollView, SearchField, Separator, Sidebar, Signal,
            Slider, Spacer, SplitView, Tab, TabView, Table, Text, TextEditor, TextField, Toggle,
            ToggleStyle, Toolbar, VStack, Window, ZStack,
        };
    }
}
