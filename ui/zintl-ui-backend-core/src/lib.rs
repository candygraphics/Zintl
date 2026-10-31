//! Platform-neutral render nodes and the storage shared by native backends.
//!
//! Views render [`RenderNode`] descriptions; each backend maps them to its
//! platform's views and reports [`Event`]s back.

mod canvas;
mod context;
mod event;
mod model;

pub mod headless;
pub mod layout;
pub mod tree;

pub use canvas::{
    Canvas2DHandler, CanvasEvent, CanvasEventContext, CanvasHost, CanvasNode, CanvasSetup, Frame,
    Invalidator, Key, KeyEvent, KeyPhase, Modifiers, NamedKey, PointerButton, PointerEvent,
    PointerPhase, Redraw, RedrawPolicy, ScrollEvent,
};
pub use context::Context;
pub use event::{Event, EventKind};
pub use headless::HeadlessBackend;
pub use model::{
    AlertButton, AlertButtonRole, AlertConfig, AlertStyle, BezelStyle, ButtonType, CommandItem,
    CommandMenu, CommandModifier, CommandRole, CommandSet, FileDialogConfig, FileDialogMode,
    ListSection, ListStyle, MenuRole, NumberFormat, PopoverEdge, ScrollAxes, Sidebar, SidebarItem,
    SidebarSearch, SidebarSection, TextAlignment, TextColor, TextStyle, TitleBar, WindowAppMenu,
    WindowStyle,
};
pub use zintl_ui_graphics as graphics;
pub use zintl_ui_graphics::{Color, Rect};
use zintl_ui_layout::{LayoutStyle, Size};

pub type MainTask = Box<dyn FnOnce() + Send + 'static>;

pub type MainTaskSender = dyn Fn(MainTask) -> bool + Send + Sync;

/// A push, toggle, checkbox, or radio button.
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    pub title: String,
    pub button_type: ButtonType,
    /// `None` draws the button without a bezel.
    pub bezel: Option<BezelStyle>,
    pub on: bool,
    pub system_image: Option<String>,
    pub key_equivalent: Option<String>,
    pub destructive: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewKind {
    Container,
    Button(Button),
    TextField {
        value: String,
        placeholder: Option<String>,
        editable: bool,
        selectable: bool,
        bordered: bool,
        draws_background: bool,
        multiline: bool,
        style: TextStyle,
    },
    /// A scrolling multiline editor that fills its parent.
    TextEditor {
        value: String,
        /// `None` uses the platform's standard editor font.
        font: Option<graphics::Font>,
        editable: bool,
    },
    Switch {
        on: bool,
    },
    Slider {
        value: f64,
        minimum: f64,
        maximum: f64,
        tick_marks: usize,
        snaps_to_ticks: bool,
    },
    PopUpButton {
        items: Vec<String>,
        selected: Option<usize>,
    },
    SegmentedControl {
        segments: Vec<String>,
        selected: Option<usize>,
    },
    ComboBox {
        items: Vec<String>,
        value: String,
        placeholder: Option<String>,
    },
    NumberField {
        value: f64,
        format: NumberFormat,
    },
    SearchField {
        value: String,
        placeholder: Option<String>,
    },
    ColorWell {
        color: Color,
    },
    /// Children are laid out in the scrollable document view.
    ScrollView {
        axes: ScrollAxes,
        /// The `(x, y)` document point restored at the top-left once the view is created.
        offset: Option<(f64, f64)>,
    },
    /// Each child is the page for the title at the same index.
    TabView {
        titles: Vec<String>,
        selected: usize,
    },
    /// A selectable list; each child is the row for the ID at the same position.
    List {
        sections: Vec<ListSection>,
        selected: Option<String>,
        style: ListStyle,
    },
    /// A view drawn by its canvas handler.
    Canvas2D {
        canvas: CanvasNode,
        redraw: RedrawPolicy,
        /// Whether a click or the handler can give the canvas keyboard focus.
        focusable: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
/// A view, window, presentation, or menu that a backend renders natively.
pub enum RenderNode {
    Window {
        sidebar: Option<Sidebar>,
        bounds: Rect,
        title: String,
        title_bar: TitleBar,
        id: Option<String>,
        style: WindowStyle,
    },
    View {
        kind: ViewKind,
        layout: LayoutStyle,
        id: Option<String>,
        enabled: bool,
        /// Receives mouse input inside the title bar area of a window whose
        /// content extends under it; other views leave that area to the window.
        title_bar_interactive: bool,
    },
    /// A contextual menu for the parent view.
    ContextMenu { items: Vec<CommandItem> },
    /// A popover anchored to the parent view; children are its content.
    Popover {
        presented: bool,
        edge: PopoverEdge,
        /// `None` sizes the popover to fit its content.
        size: Option<Size>,
    },
    /// An alert sheet on the containing window.
    Alert {
        presented: bool,
        config: AlertConfig,
    },
    /// An open or save panel sheet on the containing window.
    FileDialog {
        presented: bool,
        config: FileDialogConfig,
    },
    /// The shared color panel, connected to the containing window.
    ColorPanel {
        presented: bool,
        color: Color,
        shows_alpha: bool,
    },
    /// The application's main menu; it must be a root-level node.
    MainMenu(CommandSet),
}

impl RenderNode {
    /// Returns whether this node is presented outside its parent's layout.
    pub fn is_attachment(&self) -> bool {
        matches!(
            self,
            Self::ContextMenu { .. }
                | Self::Popover { .. }
                | Self::Alert { .. }
                | Self::FileDialog { .. }
                | Self::ColorPanel { .. }
        )
    }

    /// Keeps the canvas handler of `mounted` and reconciles this node's handler into it.
    ///
    /// Backends call this before storing an updated node so the native view
    /// keeps drawing with the handler that owns its state.
    pub fn adopt_canvas(&mut self, mounted: &RenderNode) {
        if let (
            Self::View {
                kind: ViewKind::Canvas2D { canvas, .. },
                ..
            },
            Self::View {
                kind: ViewKind::Canvas2D {
                    canvas: mounted, ..
                },
                ..
            },
        ) = (self, mounted)
            && canvas.handler_type() == mounted.handler_type()
        {
            mounted.adopt(canvas);
            *canvas = mounted.clone();
        }
    }
}

impl zintl_ui::renderer::RenderNode for RenderNode {
    type Event = Event;
    type Context<'a> = Context<'a>;

    /// Views keep their native object while only properties change.
    fn same_kind(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::View {
                    kind: ViewKind::Canvas2D { canvas: first, .. },
                    ..
                },
                Self::View {
                    kind: ViewKind::Canvas2D { canvas: second, .. },
                    ..
                },
            ) => first.handler_type() == second.handler_type(),
            (Self::View { kind: first, .. }, Self::View { kind: second, .. }) => {
                std::mem::discriminant(first) == std::mem::discriminant(second)
            }
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(usize);
