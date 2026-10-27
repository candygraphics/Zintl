//! AppKit renderer for Zintl UI trees.

mod sidebar;
pub use sidebar::{Sidebar, SidebarItem, SidebarSearch, SidebarSection};

use zintl_ui::renderer::RenderNode;
use zintl_ui::view::StoreContext;
use zintl_ui_layout::{LayoutStyle, Size};
pub use zpd_appkit::actor::WindowEventKind as AppKitEvent;
pub use zpd_appkit::ui::{
    AlertButton, AlertButtonRole, AlertConfig, AlertStyle, BezelStyle, ButtonType, Color,
    CommandItem, CommandMenu, CommandModifier, CommandRole, CommandSet, FileDialogConfig,
    FileDialogMode, MenuRole, NumberFormat, PopoverEdge, ScrollAxes, WindowAppMenu, WindowStyle,
};

pub type MainTask = Box<dyn FnOnce() + Send + 'static>;

pub type MainTaskSender = dyn Fn(MainTask) -> bool + Send + Sync;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// An `NSButton` configuration.
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Window {
        sidebar: Option<Sidebar>,
        bounds: Rect,
        title: String,
        full_size_content_view: bool,
        id: Option<String>,
        style: WindowStyle,
    },
    View {
        kind: ViewKind,
        layout: LayoutStyle,
        id: Option<String>,
        enabled: bool,
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

impl NodeKind {
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
}

pub trait AppKitRenderNode: RenderNode {
    fn create_context<'a>(
        store_context: StoreContext<'a>,
        perform_main: Option<&'a MainTaskSender>,
    ) -> Self::Context<'a>;
    fn appkit_node(&self) -> NodeKind;
    fn appkit_event(event: AppKitEvent) -> Self::Event;

    /// Returns the main menu installed while the tree declares none.
    fn default_main_menu() -> Option<CommandSet> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

#[cfg(target_os = "macos")]
mod backend;

#[cfg(target_os = "macos")]
pub use backend::{AppError, AppKitBackend, run_composer};
