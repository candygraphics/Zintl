use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

use messageloop_dispatcherqueue::{MessageLoopError, Sender};
use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
use zintl_ui::view::StoreContext;
use zintl_ui_backend_core::layout::{HostLayout, LayoutSnapshot};
use zintl_ui_backend_core::tree::{ErrorSlot, MainTasks, NodeTree};
use zintl_ui_backend_core::{
    AlertButtonRole, AlertConfig, Color, CommandSet, Context, Event, FileDialogConfig,
    FileDialogMode, MainTask, NodeId, PopoverEdge, Rect, RenderNode, Sidebar, TitleBar, ViewKind,
    WindowStyle,
};
use zintl_ui_layout::{LayoutError, Size};
use zpd_winui3::{
    ApplicationContext, ApplicationCreateError, AsElement, BoolProperty, Control, ControlEvent,
    ControlKind, DialogConfig, DialogResult, FileDialogOptions, MenuBar, NavigationItem, Placement,
    Popover, Window,
};

mod app;
mod attachment;
mod menu;
mod native;

pub use app::run_composer;
use app::{Message, MessageSender};
use attachment::{Attachment, ContextMenu};
use menu::Menus;
use native::{CanvasHost, Glyph, HostContent, NativeNode, RouteCell};

/// Inset between a popover's edge and its content.
const POPOVER_PADDING: f32 = 12.0;

#[derive(Debug)]
pub enum AppError {
    Application(ApplicationCreateError),
    /// A native call failed; `message` is the description WinUI gave.
    WinUI {
        error: zpd_winui3::Error,
        message: String,
    },
    Layout(LayoutError),
    MessageLoop(MessageLoopError),
    NoWindow,
    InvalidTree,
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Application(error) => error.fmt(formatter),
            Self::WinUI { error, message } if message.is_empty() => error.fmt(formatter),
            Self::WinUI { error, message } => write!(formatter, "{error}: {message}"),
            Self::Layout(error) => error.fmt(formatter),
            Self::MessageLoop(error) => error.fmt(formatter),
            Self::NoWindow => formatter.write_str("the rendered tree does not contain a window"),
            Self::InvalidTree => formatter.write_str("a window cannot be nested in a view"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Application(error) => Some(error),
            Self::WinUI { error, .. } => Some(error),
            Self::Layout(error) => Some(error),
            Self::MessageLoop(error) => Some(error),
            Self::NoWindow | Self::InvalidTree => None,
        }
    }
}

impl From<zpd_winui3::Error> for AppError {
    fn from(error: zpd_winui3::Error) -> Self {
        Self::WinUI {
            error,
            message: zpd_winui3::Error::last_message(),
        }
    }
}

impl From<LayoutError> for AppError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

/// Native state of a rendered Window node.
struct WindowState {
    native: Window,
    /// Hosts the window's top-level views.
    canvas: Control,
    host: Rc<CanvasHost>,
    /// The sidebar, which then contains `canvas`.
    navigation: Option<Control>,
    style: WindowStyle,
    applied_bounds: Option<Rect>,
    applied_title: Option<String>,
    applied_title_bar: Option<TitleBar>,
    /// `None` until the sidebar declaration has been applied once.
    applied_sidebar: Option<Option<Sidebar>>,
    applied_menu: Option<u64>,
    /// Set when the window closed; it is not recreated afterwards.
    closed: Rc<Cell<bool>>,
}

/// WinUI state attached to a rendered node.
#[derive(Default)]
struct NodeState {
    native: Option<Rc<NativeNode>>,
    window: Option<WindowState>,
    attachment: Option<Attachment>,
    /// The attachment declaration last applied to `attachment`.
    applied: Option<RenderNode>,
    /// The route reported by this node's window or attachment.
    route: RouteCell,
}

impl NodeState {
    fn is_closed_window(&self) -> bool {
        self.window.as_ref().is_some_and(|state| state.closed.get())
    }
}

pub struct WinUI3Backend {
    tree: NodeTree<NodeState>,
    structure_dirty: bool,
    layout_dirty: bool,
    pending_error: ErrorSlot<AppError>,
    main_tasks: MainTasks,
    main_menu: Option<CommandSet>,
    /// Installed while the tree declares no main menu.
    default_main_menu: Option<CommandSet>,
    menu_bar: MenuBar,
    /// Increments whenever `menu_bar` changes.
    menu_version: u64,
    opened_window: bool,
}

impl Default for WinUI3Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl WinUI3Backend {
    pub fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            structure_dirty: true,
            layout_dirty: true,
            pending_error: ErrorSlot::default(),
            main_tasks: MainTasks::new(),
            main_menu: None,
            default_main_menu: None,
            menu_bar: MenuBar::default(),
            menu_version: 0,
            opened_window: false,
        }
    }

    /// Sets the main menu shown while the tree declares none.
    pub fn set_default_main_menu(&mut self, commands: CommandSet) {
        self.default_main_menu = Some(commands);
    }

    fn connect_main_tasks(&self, sender: impl Fn(MainTask) -> bool + Send + Sync + 'static) {
        self.main_tasks.connect(sender);
    }

    pub fn value(&self, id: NodeId) -> Option<&RenderNode> {
        self.tree.value(id)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.tree.children(id)
    }

    /// Returns the Composer route associated with a rendered backend node.
    pub fn event_route(&self, id: NodeId) -> Option<EventRouteId> {
        self.tree.node(id).event_route
    }

    fn state(&self, id: NodeId) -> &NodeState {
        &self.tree.node(id).state
    }

    fn state_mut(&mut self, id: NodeId) -> &mut NodeState {
        &mut self.tree.node_mut(id).state
    }

    fn kind(&self, id: NodeId) -> RenderNode {
        self.tree.kind(id)
    }

    fn window_native(&self, window: NodeId) -> &Window {
        &self
            .state(window)
            .window
            .as_ref()
            .expect("attachments synchronize inside a live window")
            .native
    }

    /// Drops native state below a window whose native object was replaced.
    fn reset_natives(&mut self, id: NodeId) {
        *self.state_mut(id) = NodeState::default();
        for child in self.children(id).to_vec() {
            self.reset_natives(child);
        }
    }

    /// Returns whether the app should end because its last normal window is gone.
    fn should_terminate(&self) -> bool {
        self.opened_window && !self.tree.has_normal_window(NodeState::is_closed_window)
    }

    fn close_window(&self, window: NodeId) {
        if let Some(state) = self
            .tree
            .contains(window)
            .then(|| self.state(window).window.as_ref())
            .flatten()
        {
            if let Err(error) = state.native.close() {
                self.pending_error.record(error.into());
            }
        }
    }

    fn main_menu_route(&self) -> Option<EventRouteId> {
        self.tree.main_menu_route()
    }

    fn synchronize(
        &mut self,
        application: &ApplicationContext,
        sender: &MessageSender,
    ) -> Result<(), AppError> {
        self.pending_error.take()?;
        let commands = self.tree.main_menu(self.default_main_menu.as_ref());
        if self.main_menu.as_ref() != Some(&commands) {
            self.menu_bar = Menus::menu_bar(&commands);
            self.main_menu = Some(commands);
            self.menu_version += 1;
        }
        let window_ids = self.tree.window_ids(NodeState::is_closed_window);
        if window_ids.is_empty() && !self.opened_window {
            return Err(AppError::NoWindow);
        }
        let rebuild_structure = self.structure_dirty;
        for window_id in window_ids {
            self.synchronize_window(application, sender, window_id, rebuild_structure)?;
        }
        self.structure_dirty = false;
        self.layout_dirty = false;
        self.pending_error.take()
    }

    fn create_window(
        &self,
        application: &ApplicationContext,
        sender: &MessageSender,
        route: &RouteCell,
        style: WindowStyle,
    ) -> Result<WindowState, AppError> {
        let native = application.create_styled_window(match style {
            WindowStyle::Normal => zpd_winui3::WindowStyle::Normal,
            WindowStyle::Panel => zpd_winui3::WindowStyle::Panel,
        })?;
        let canvas = Control::new(ControlKind::Canvas)?;
        let host = CanvasHost::new(self.pending_error.clone());
        let resized = host.clone();
        canvas.set_event_handler(move |event| {
            if let ControlEvent::SizeChanged { width, height } = event {
                resized.resize(Size::new(width as f32, height as f32));
            }
        })?;
        native.set_content(Some(&canvas))?;
        let closed = Rc::new(Cell::new(false));
        let (closed_flag, route, sender) = (closed.clone(), route.clone(), sender.clone());
        native.set_closed(move || {
            closed_flag.set(true);
            let _ = sender.send(Message::WindowClosed { route: route.get() });
        })?;
        Ok(WindowState {
            native,
            canvas,
            host,
            navigation: None,
            style,
            applied_bounds: None,
            applied_title: None,
            applied_title_bar: None,
            applied_sidebar: None,
            applied_menu: None,
            closed,
        })
    }

    fn synchronize_window(
        &mut self,
        application: &ApplicationContext,
        sender: &MessageSender,
        window_id: NodeId,
        rebuild_structure: bool,
    ) -> Result<(), AppError> {
        let RenderNode::Window {
            bounds,
            title,
            title_bar,
            sidebar,
            style,
            ..
        } = self.kind(window_id)
        else {
            return Ok(());
        };

        // The presenter cannot change, so a new style needs a new window.
        if self
            .state(window_id)
            .window
            .as_ref()
            .is_some_and(|state| state.style != style)
        {
            self.reset_natives(window_id);
        }
        let new_window = self.state(window_id).window.is_none();
        if new_window {
            let route = self.state(window_id).route.clone();
            route.set(self.event_route(window_id));
            let state = self.create_window(application, sender, &route, style)?;
            self.state_mut(window_id).window = Some(state);
            self.opened_window = true;
        }
        let menu_bar = (style == WindowStyle::Normal).then(|| self.menu_bar.clone());
        let menu_version = self.menu_version;
        let route = self.state(window_id).route.clone();
        let state = self
            .state_mut(window_id)
            .window
            .as_mut()
            .expect("a synchronized window has native state");
        if state.applied_bounds != Some(bounds) {
            state
                .native
                .move_and_resize(bounds.x, bounds.y, bounds.width, bounds.height)?;
            state.applied_bounds = Some(bounds);
        }
        if state.applied_title.as_ref() != Some(&title) {
            state.native.set_title(&title)?;
            state.applied_title = Some(title);
        }
        if state.applied_title_bar != Some(title_bar) {
            state
                .native
                .set_extends_content_into_title_bar(title_bar != TitleBar::Standard)?;
            state.applied_title_bar = Some(title_bar);
        }
        if state.applied_sidebar.as_ref() != Some(&sidebar) {
            Self::apply_sidebar(state, sidebar.as_ref(), sender, &route)?;
            state.applied_sidebar = Some(sidebar);
        }
        if let Some(menu_bar) = menu_bar
            && state.applied_menu != Some(menu_version)
        {
            if menu_bar.items.is_empty() {
                state.native.clear_menu_bar()?;
            } else {
                let sender = sender.clone();
                state.native.set_menu_bar(&menu_bar, move |id| {
                    let _ = sender.send(Message::Command {
                        id: id.into(),
                        window: window_id,
                    });
                })?;
            }
            state.applied_menu = Some(menu_version);
        }

        let recreated = self.materialize_children(sender, window_id)?;
        let rebuild_structure = rebuild_structure || new_window || recreated;
        let state = self
            .state(window_id)
            .window
            .as_ref()
            .expect("a synchronized window has native state");
        if rebuild_structure {
            self.attach_children(&state.canvas, window_id)?;
        }
        let update_layout = self.layout_dirty || rebuild_structure;
        if update_layout {
            state.host.set_content(HostContent::Host(HostLayout {
                children: self.content_snapshots(window_id)?,
                padding: 0.0,
            }));
            self.pending_error.take()?;
        }
        if new_window {
            state.native.activate()?;
        }
        self.synchronize_attachments(
            application,
            sender,
            window_id,
            window_id,
            rebuild_structure,
            update_layout,
        )
    }

    fn apply_sidebar(
        state: &mut WindowState,
        sidebar: Option<&Sidebar>,
        sender: &MessageSender,
        route: &RouteCell,
    ) -> Result<(), AppError> {
        let Some(sidebar) = sidebar else {
            if let Some(navigation) = state.navigation.take() {
                navigation.set_children(&[])?;
                state.native.set_content(Some(&state.canvas))?;
            }
            return Ok(());
        };
        if state.navigation.is_none() {
            let navigation = Control::new(ControlKind::NavigationView)?;
            let (sender, route) = (sender.clone(), route.clone());
            navigation.set_event_handler(move |event| {
                if let (Some(route), Some(event)) = (route.get(), NativeNode::event(event)) {
                    let _ = sender.send(Message::Event { route, event });
                }
            })?;
            // WinUI moves an element only within one XAML tree, so the
            // navigation view joins the window before it adopts the canvas.
            state.native.set_content(Some(&navigation))?;
            navigation.set_children(&[state.canvas.as_element()])?;
            state.navigation = Some(navigation);
        }
        let navigation = state.navigation.as_ref().expect("created above");
        let items = sidebar
            .sections
            .iter()
            .flat_map(|section| {
                let header = section.title.clone().map(NavigationItem::Header);
                header.into_iter().chain(section.items.iter().map(|item| {
                    NavigationItem::Item {
                        id: item.id.clone(),
                        title: item.title.clone(),
                        glyph: item
                            .system_image
                            .as_deref()
                            .and_then(Glyph::of)
                            .map(str::to_owned),
                    }
                }))
            })
            .collect::<Vec<_>>();
        navigation.set_navigation_items(&items)?;
        navigation.select_navigation_item(sidebar.selected_id.as_deref())?;
        navigation.set_navigation_search(sidebar.search.as_ref().map(|search| {
            (
                search.value.as_str(),
                search.placeholder.as_deref().unwrap_or(""),
            )
        }))?;
        Ok(())
    }

    /// Creates missing natives; returns whether any were created.
    fn materialize_children(
        &mut self,
        sender: &MessageSender,
        parent: NodeId,
    ) -> Result<bool, AppError> {
        let mut created = false;
        for child in self.children(parent).to_vec() {
            if let RenderNode::View { kind, enabled, .. } = self.kind(child)
                && self.state(child).native.is_none()
            {
                let native = NativeNode::create(&kind, sender, &self.pending_error)?;
                if !enabled {
                    native.set_enabled(false)?;
                }
                native.route.set(self.event_route(child));
                self.state_mut(child).native = Some(Rc::new(native));
                created = true;
            }
            created |= self.materialize_children(sender, child)?;
        }
        Ok(created)
    }

    /// Returns the natives of `parent`'s view children; attachments are presented separately.
    fn view_children(&self, parent: NodeId) -> Vec<(NodeId, Rc<NativeNode>)> {
        self.children(parent)
            .iter()
            .filter_map(|child| Some((*child, self.state(*child).native.clone()?)))
            .collect()
    }

    fn attach_children(&self, parent: &Control, parent_id: NodeId) -> Result<(), AppError> {
        let children = self.view_children(parent_id);
        let elements = children
            .iter()
            .map(|(_, native)| native.control.as_element())
            .collect::<Vec<_>>();
        parent.set_children(&elements)?;
        for (child, native) in &children {
            if matches!(
                native.control.kind(),
                ControlKind::Canvas | ControlKind::ScrollViewer | ControlKind::TabView
            ) {
                self.attach_children(&native.control, *child)?;
            }
        }
        Ok(())
    }

    fn content_snapshots(
        &self,
        parent: NodeId,
    ) -> Result<Vec<LayoutSnapshot<NativeNode>>, AppError> {
        self.children(parent)
            .iter()
            .filter(|child| !self.kind(**child).is_attachment())
            .map(|child| self.build_layout_snapshot(*child))
            .collect()
    }

    fn build_layout_snapshot(&self, id: NodeId) -> Result<LayoutSnapshot<NativeNode>, AppError> {
        let RenderNode::View {
            kind,
            layout: mut style,
            ..
        } = self.kind(id)
        else {
            return Err(AppError::InvalidTree);
        };
        style.minimum_size = NativeNode::minimum_size(&kind, style.minimum_size);
        let native = self
            .state(id)
            .native
            .as_ref()
            .expect("materialized view nodes have native views")
            .clone();
        let content = self.content_snapshots(id)?;
        let (children, hosted) = match kind {
            ViewKind::ScrollView { .. } | ViewKind::TabView { .. } | ViewKind::List { .. } => {
                (Vec::new(), content)
            }
            _ => (content, Vec::new()),
        };
        Ok(LayoutSnapshot {
            style,
            native,
            children,
            hosted,
        })
    }

    /// Presents attachments below `id` in tree order, parents first.
    fn synchronize_attachments(
        &mut self,
        application: &ApplicationContext,
        sender: &MessageSender,
        window: NodeId,
        id: NodeId,
        rebuild: bool,
        relayout: bool,
    ) -> Result<(), AppError> {
        for child in self.children(id).to_vec() {
            if self.kind(child).is_attachment() {
                self.synchronize_attachment(application, sender, window, child, rebuild, relayout)?;
            }
            self.synchronize_attachments(application, sender, window, child, rebuild, relayout)?;
        }
        Ok(())
    }

    fn synchronize_attachment(
        &mut self,
        application: &ApplicationContext,
        sender: &MessageSender,
        window: NodeId,
        id: NodeId,
        rebuild: bool,
        relayout: bool,
    ) -> Result<(), AppError> {
        let kind = self.kind(id);
        let route = self.state(id).route.clone();
        route.set(self.event_route(id));
        let report = {
            let sender = sender.clone();
            let route = route.clone();
            move |event: Event| {
                if let Some(route) = route.get() {
                    let _ = sender.send(Message::Event { route, event });
                }
            }
        };
        let anchor = self
            .tree
            .node(id)
            .parent
            .and_then(|parent| self.state(parent).native.clone());
        let applied = self.state(id).applied.clone();
        let was_presented = applied.as_ref().is_some_and(RenderNode::is_presented);
        let has_sheet = self.window_native(window).has_sheet();
        // A window shows one sheet at a time; retry after the current one ends.
        if kind.is_presented_sheet() && !was_presented && has_sheet {
            return Ok(());
        }
        match &kind {
            RenderNode::ContextMenu { items } => {
                let Some(anchor) = anchor else {
                    return Ok(());
                };
                if self.state(id).attachment.is_none() || applied.as_ref() != Some(&kind) {
                    // Drop the old menu first so its teardown cannot clear the new one.
                    self.state_mut(id).attachment = None;
                    anchor
                        .control
                        .set_context_menu(&Menus::items(items), move |id| {
                            report(Event::MenuItemSelected { id: id.into() });
                        })?;
                    self.state_mut(id).attachment = Some(Attachment::ContextMenu {
                        _menu: ContextMenu { anchor },
                    });
                }
            }
            RenderNode::Popover {
                presented,
                edge,
                size,
            } => {
                let Some(anchor) = anchor else {
                    return Ok(());
                };
                let created = self.state(id).attachment.is_none();
                if created {
                    let content = Control::new(ControlKind::Canvas)?;
                    let host = CanvasHost::new(self.pending_error.clone());
                    let popover = Popover::new(&content, move || report(Event::PopoverClosed))?;
                    self.state_mut(id).attachment = Some(Attachment::Popover {
                        popover,
                        content,
                        host,
                    });
                }
                let Some(Attachment::Popover {
                    popover,
                    content,
                    host,
                }) = &self.state(id).attachment
                else {
                    unreachable!("the popover attachment was created above");
                };
                if created || rebuild {
                    self.attach_children(content, id)?;
                }
                if created || rebuild || relayout {
                    let layout = HostLayout {
                        children: self.content_snapshots(id)?,
                        padding: POPOVER_PADDING,
                    };
                    let fitted = match size {
                        Some(size) => *size,
                        None => layout.fitting_size()?,
                    };
                    let fitted = Size::new(fitted.width.max(1.0), fitted.height.max(1.0));
                    content.set_width(f64::from(fitted.width))?;
                    content.set_height(f64::from(fitted.height))?;
                    host.set_content(HostContent::Host(layout));
                    host.resize(fitted);
                }
                if *presented && (created || !was_presented) && !popover.is_open() {
                    popover.show(&anchor.control, Self::placement(*edge))?;
                } else if !*presented && popover.is_open() {
                    popover.hide()?;
                }
            }
            RenderNode::Alert { presented, config } => {
                if *presented && !was_presented {
                    self.state_mut(id).attachment = None;
                    let (dialog, indices) = Self::dialog(config);
                    let native =
                        self.window_native(window)
                            .show_dialog(&dialog, move |result| {
                                let button = match result {
                                    DialogResult::Primary => indices[0],
                                    DialogResult::Secondary => indices[1],
                                    DialogResult::Close => indices[2],
                                    DialogResult::None => None,
                                };
                                report(Event::AlertResponse { button });
                            })?;
                    self.state_mut(id).attachment = Some(Attachment::Alert { _dialog: native });
                } else if !*presented {
                    // Dropping a dialog that is still open hides it.
                    self.state_mut(id).attachment = None;
                }
            }
            RenderNode::FileDialog { presented, config } => {
                if *presented && !was_presented {
                    self.window_native(window).show_file_dialog(
                        &Self::file_dialog(config),
                        move |paths| {
                            report(Event::FileDialogCompleted {
                                paths: paths
                                    .map(|paths| paths.into_iter().map(PathBuf::from).collect()),
                            });
                        },
                    )?;
                    self.state_mut(id).attachment = Some(Attachment::FileDialog);
                } else if !*presented {
                    self.state_mut(id).attachment = None;
                }
            }
            RenderNode::ColorPanel {
                presented,
                color,
                shows_alpha,
            } => {
                if *presented && !was_presented {
                    self.state_mut(id).attachment = None;
                    let attachment = Self::color_panel(application, *color, *shows_alpha, report)?;
                    self.state_mut(id).attachment = Some(attachment);
                } else if *presented {
                    if let Some(Attachment::ColorPanel { picker, .. }) = &self.state(id).attachment
                    {
                        let current = picker.color()?;
                        let current =
                            Color::rgba(current.red, current.green, current.blue, current.alpha);
                        if !Self::colors_match(current, *color) {
                            picker.set_color(Self::native_color(*color))?;
                        }
                    }
                } else {
                    self.state_mut(id).attachment = None;
                }
            }
            _ => {}
        }
        self.state_mut(id).applied = Some(kind);
        Ok(())
    }

    fn placement(edge: PopoverEdge) -> Placement {
        match edge {
            PopoverEdge::Leading => Placement::Left,
            PopoverEdge::Trailing => Placement::Right,
            PopoverEdge::Top => Placement::Top,
            PopoverEdge::Bottom => Placement::Bottom,
        }
    }

    /// Maps alert buttons to dialog buttons and the alert index each reports.
    ///
    /// The first non-cancel button is primary and answers Enter; the first
    /// cancel button closes the dialog.
    fn dialog(config: &AlertConfig) -> (DialogConfig, [Option<usize>; 3]) {
        let mut dialog = DialogConfig {
            title: config.title.clone(),
            message: config.message.clone(),
            ..DialogConfig::default()
        };
        let mut indices = [None; 3];
        for (index, button) in config.buttons.iter().enumerate() {
            let slot = match button.role {
                AlertButtonRole::Cancel if indices[2].is_none() => 2,
                AlertButtonRole::Cancel => continue,
                _ if indices[0].is_none() => 0,
                _ if indices[1].is_none() => 1,
                _ => continue,
            };
            indices[slot] = Some(index);
            let title = button.title.clone();
            match slot {
                0 => dialog.primary = title,
                1 => dialog.secondary = title,
                _ => dialog.close = title,
            }
        }
        if config.buttons.is_empty() {
            dialog.close = "OK".into();
            indices[2] = Some(0);
        }
        if indices[0].is_some() {
            dialog.default_button = Some(DialogResult::Primary);
        }
        (dialog, indices)
    }

    fn file_dialog(config: &FileDialogConfig) -> FileDialogOptions {
        let mode = match config.mode {
            FileDialogMode::Save => zpd_winui3::FileDialogMode::Save,
            FileDialogMode::Open if config.can_choose_directories && !config.can_choose_files => {
                zpd_winui3::FileDialogMode::Folder
            }
            FileDialogMode::Open => zpd_winui3::FileDialogMode::Open,
        };
        FileDialogOptions {
            mode,
            multiple: config.allows_multiple,
            file_name: config.file_name.clone(),
            commit_text: config.prompt.clone(),
            extensions: config.allowed_extensions.clone(),
        }
    }

    /// Opens a utility window with a color picker connected to `report`.
    fn color_panel(
        application: &ApplicationContext,
        color: Color,
        shows_alpha: bool,
        report: impl Fn(Event) + 'static,
    ) -> Result<Attachment, AppError> {
        let report = Rc::new(report);
        let window = application.create_styled_window(zpd_winui3::WindowStyle::Panel)?;
        window.set_title("Colors")?;
        window.move_and_resize(200.0, 200.0, 360.0, 520.0)?;
        let picker = Control::new(ControlKind::ColorPicker)?;
        picker.set_bool(BoolProperty::Alpha, shows_alpha)?;
        picker.set_color(Self::native_color(color))?;
        let changed = report.clone();
        picker.set_event_handler(move |event| {
            if let Some(event @ Event::ColorChanged { .. }) = NativeNode::event(event) {
                changed(event);
            }
        })?;
        window.set_content(Some(&picker))?;
        window.set_closed(move || report(Event::ColorPanelClosed))?;
        window.activate()?;
        Ok(Attachment::ColorPanel {
            _window: window,
            picker,
        })
    }

    fn native_color(color: Color) -> zpd_winui3::Color {
        zpd_winui3::Color {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha: color.alpha,
        }
    }

    /// Compares colors after a round trip through 8-bit channels.
    fn colors_match(first: Color, second: Color) -> bool {
        const TOLERANCE: f64 = 1.0 / 255.0;
        [
            first.red - second.red,
            first.green - second.green,
            first.blue - second.blue,
            first.alpha - second.alpha,
        ]
        .iter()
        .all(|difference| difference.abs() <= TOLERANCE)
    }

    /// Applies a changed declaration; a different control kind is recreated.
    fn update_native(
        &mut self,
        id: NodeId,
        old: &RenderNode,
        new: &RenderNode,
    ) -> Result<(), AppError> {
        let Some(native) = self.state(id).native.clone() else {
            return Ok(());
        };
        let (
            RenderNode::View {
                kind: old_kind,
                enabled: old_enabled,
                ..
            },
            RenderNode::View {
                kind: new_kind,
                enabled: new_enabled,
                ..
            },
        ) = (old, new)
        else {
            return Ok(());
        };
        if !native.matches(new_kind) {
            self.state_mut(id).native = None;
            self.structure_dirty = true;
            return Ok(());
        }
        if old_enabled != new_enabled {
            native.set_enabled(*new_enabled)?;
        }
        native.apply(Some(old_kind), new_kind)
    }
}

impl RenderBackend<RenderNode> for WinUI3Backend {
    type NodeId = NodeId;

    fn create_context<'a>(&'a self, store_context: StoreContext<'a>) -> Context<'a> {
        Context::new(store_context, Some(self.main_tasks.sender()))
    }

    fn root(&self) -> Self::NodeId {
        NodeTree::<NodeState>::ROOT
    }

    fn create(&mut self, value: &RenderNode, event_route: Option<EventRouteId>) -> Self::NodeId {
        self.structure_dirty = true;
        self.layout_dirty = true;
        self.tree.create(value, event_route)
    }

    fn update(&mut self, id: Self::NodeId, value: &RenderNode) {
        let old = self.kind(id);
        let mut new = value.clone();
        new.adopt_canvas(&old);
        if let Err(error) = self.update_native(id, &old, &new) {
            self.pending_error.record(error);
        }
        self.tree.node_mut(id).value = Some(new);
        self.layout_dirty = true;
    }

    fn set_event_route(&mut self, id: Self::NodeId, event_route: Option<EventRouteId>) {
        self.tree.node_mut(id).event_route = event_route;
        let state = self.state(id);
        state.route.set(event_route);
        if let Some(native) = &state.native {
            native.route.set(event_route);
        }
    }

    fn insert_child(&mut self, parent: Self::NodeId, index: usize, child: Self::NodeId) {
        self.tree.insert_child(parent, index, child);
        self.structure_dirty = true;
        self.layout_dirty = true;
    }

    fn remove(&mut self, id: Self::NodeId) {
        // Dropping native state closes windows and releases controls.
        self.tree.remove_subtree(id, &mut drop);
        self.structure_dirty = true;
        self.layout_dirty = true;
    }

    fn move_child(&mut self, parent: Self::NodeId, index: usize, child: Self::NodeId) {
        if !self.tree.is_at(parent, index, child) {
            self.insert_child(parent, index, child);
        }
    }
}

#[cfg(test)]
mod tests;
