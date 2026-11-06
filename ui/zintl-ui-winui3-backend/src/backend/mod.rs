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
    AlertButtonRole, AlertConfig, Color, CommandSet, Context, DocumentState, Event,
    FileDialogConfig, FileDialogMode, MainTask, NodeId, PopoverEdge, Rect, RenderNode, Sidebar,
    TitleBar, Toolbar, ToolbarItem, ViewKind, WindowStyle,
};
use zintl_ui_layout::{LayoutError, Size};
use zpd_winui3::{
    ApplicationContext, ApplicationCreateError, AsElement, BoolProperty, Control, ControlEvent,
    ControlKind, DialogConfig, DialogResult, FileDialogOptions, ListEntry, MenuBar, NavigationItem,
    Placement, Popover, Window, WindowEvent,
};

mod app;
mod attachment;
mod canvas;
mod menu;
mod native;
#[cfg(feature = "wgpu")]
mod wgpu;

pub use app::run_composer;
use app::{Message, MessageSender};
use attachment::{Attachment, ContextMenu};
use menu::Menus;
use native::{CanvasHost, Environment, Glyph, HostContent, NativeNode, NativeView, RouteCell};

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
    /// A wgpu canvas could not reach the GPU; it stays blank.
    #[cfg(feature = "wgpu")]
    Wgpu(zintl_ui_backend_core::WgpuError),
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
            #[cfg(feature = "wgpu")]
            Self::Wgpu(error) => error.fmt(formatter),
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
            #[cfg(feature = "wgpu")]
            Self::Wgpu(error) => Some(error),
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

/// GPU objects every wgpu canvas of the app shares.
///
/// The app owns them instead of a thread-local, so they are released while the
/// app exits rather than while thread-locals are destroyed.
#[derive(Clone, Default)]
pub(super) struct GpuSlot {
    #[cfg(feature = "wgpu")]
    context: Rc<std::cell::RefCell<Option<zintl_ui_backend_core::WgpuContext>>>,
}

impl GpuSlot {
    /// Drops the app's reference; canvases that are still mounted keep theirs.
    pub(super) fn release(&self) {
        #[cfg(feature = "wgpu")]
        drop(self.context.borrow_mut().take());
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
    /// `None` until the toolbar declaration has been applied once.
    applied_toolbar: Option<Option<Toolbar>>,
    applied_document: Option<DocumentState>,
    /// The close request count already handled; a higher count closes the window.
    applied_close_requests: u64,
    applied_menu: Option<u64>,
    /// Whether closing reports a close request instead.
    confirms_close: Rc<Cell<bool>>,
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
    gpu: GpuSlot,
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
            gpu: GpuSlot::default(),
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

    fn window_state(&self, window: NodeId) -> Option<&WindowState> {
        self.tree
            .contains(window)
            .then(|| self.state(window).window.as_ref())
            .flatten()
    }

    /// Closes `window`, or returns its route when it reports a close request instead.
    fn close_window(&self, window: NodeId) -> Option<EventRouteId> {
        let state = self.window_state(window)?;
        if state.confirms_close.get() {
            return self.event_route(window);
        }
        if let Err(error) = state.native.close() {
            self.pending_error.record(error.into());
        }
        None
    }

    /// Returns the routes of open windows that confirm closing, which quitting asks first.
    fn confirming_windows(&self) -> Vec<EventRouteId> {
        self.tree
            .window_ids(NodeState::is_closed_window)
            .into_iter()
            .filter(|window| {
                self.window_state(*window)
                    .is_some_and(|state| state.confirms_close.get())
            })
            .filter_map(|window| self.event_route(window))
            .collect()
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
        close_requests: u64,
    ) -> Result<WindowState, AppError> {
        let native = application.create_styled_window(match style {
            WindowStyle::Normal => zpd_winui3::WindowStyle::Normal,
            WindowStyle::Panel => zpd_winui3::WindowStyle::Panel,
        })?;
        let canvas = Control::new(ControlKind::Canvas)?;
        let host = CanvasHost::new(self.pending_error.clone());
        let resized = host.clone();
        canvas.set_event_handler(move |event| {
            if let ControlEvent::SizeChanged {
                width,
                height,
                inset,
            } = event
            {
                resized.resize(Size::new(width as f32, height as f32), inset as f32);
            }
        })?;
        native.set_content(Some(&canvas))?;
        let (events, event_route) = (sender.clone(), route.clone());
        native.set_event_handler(move |event| {
            let event = match event {
                WindowEvent::ToolbarClicked(id) => Event::ToolbarItemClicked { id },
                WindowEvent::ToolbarSearchChanged(value) => Event::ToolbarSearchChanged { value },
                WindowEvent::CloseRequested => Event::WindowCloseRequested,
                WindowEvent::Undo => Event::Undo,
                WindowEvent::Redo => Event::Redo,
            };
            if let Some(route) = event_route.get() {
                let _ = events.send(Message::Event { route, event });
            }
        })?;
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
            applied_toolbar: None,
            applied_document: None,
            // A window created after close requests starts with them handled.
            applied_close_requests: close_requests,
            applied_menu: None,
            confirms_close: Rc::new(Cell::new(false)),
            closed,
        })
    }

    /// The window title, marked while the document has unsaved changes.
    fn displayed_title(title: &str, document: &DocumentState) -> String {
        if document.edited {
            format!("*{title}")
        } else {
            title.to_owned()
        }
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
            toolbar,
            autosave,
            document,
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
            let state =
                self.create_window(application, sender, &route, style, document.close_requests)?;
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
        // A saved placement replaces the declared bounds on a new window.
        if new_window && let Some(name) = &autosave {
            state.native.set_autosave(name)?;
        }
        let title = Self::displayed_title(&title, &document);
        if state.applied_title.as_ref() != Some(&title) {
            state.native.set_title(&title)?;
            state.applied_title = Some(title);
        }
        if state.applied_title_bar != Some(title_bar) {
            state
                .native
                .set_extends_content_into_title_bar(title_bar != TitleBar::Standard)?;
            state.host.set_inset(state.canvas.title_bar_inset() as f32);
            state.applied_title_bar = Some(title_bar);
        }
        if state.applied_sidebar.as_ref() != Some(&sidebar) {
            Self::apply_sidebar(state, sidebar.as_ref(), sender, &route)?;
            state.applied_sidebar = Some(sidebar);
        }
        if state.applied_toolbar.as_ref() != Some(&toolbar) {
            Self::apply_toolbar(&state.native, toolbar.as_ref())?;
            state.applied_toolbar = Some(toolbar);
        }
        if state.applied_document != Some(document) {
            state.native.set_document(
                document.confirms_close,
                document.undo.map(|undo| (undo.can_undo, undo.can_redo)),
            )?;
            state.confirms_close.set(document.confirms_close);
            state.applied_document = Some(document);
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
        if document.close_requests > state.applied_close_requests {
            state.applied_close_requests = document.close_requests;
            // A requested close does not ask again; closing reports the window as closed.
            state.native.set_document(false, None)?;
            state.confirms_close.set(false);
            state.native.close()?;
            return Ok(());
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
            // Views that take clicks under the title bar are collected again after each layout.
            let mut passthrough = Vec::new();
            self.title_bar_views(window_id, &mut passthrough);
            let elements = passthrough
                .iter()
                .map(|native| native.view.as_element())
                .collect::<Vec<_>>();
            state.native.set_title_bar_passthrough(&elements)?;
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

    /// Collects views below `id` that take clicks under the title bar; canvases decide per point.
    fn title_bar_views(&self, id: NodeId, views: &mut Vec<Rc<NativeNode>>) {
        for child in self.children(id) {
            if let Some(native) = &self.state(*child).native
                && (native.title_bar_interactive.get()
                    || matches!(native.view, NativeView::Canvas { .. }))
            {
                views.push(native.clone());
            }
            self.title_bar_views(*child, views);
        }
    }

    fn apply_toolbar(window: &Window, toolbar: Option<&Toolbar>) -> Result<(), AppError> {
        let items = toolbar.map_or(&[][..], |toolbar| &toolbar.items);
        let items = items
            .iter()
            .map(|item| match item {
                ToolbarItem::Button {
                    id,
                    label,
                    system_image,
                    enabled,
                } => zpd_winui3::ToolbarItem::Button {
                    id,
                    label,
                    glyph: Glyph::of(system_image),
                    enabled: *enabled,
                },
                ToolbarItem::Search { value, placeholder } => zpd_winui3::ToolbarItem::Search {
                    value,
                    placeholder: placeholder.as_deref().unwrap_or(""),
                },
                ToolbarItem::FlexibleSpace => zpd_winui3::ToolbarItem::FlexibleSpace,
            })
            .collect::<Vec<_>>();
        Ok(window.set_toolbar(&items)?)
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
                        badge: item.badge.clone(),
                        menu: Menus::items(&item.menu),
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
            if let RenderNode::View {
                kind,
                id,
                enabled,
                title_bar_interactive,
                help,
                ..
            } = self.kind(child)
                && self.state(child).native.is_none()
            {
                let environment = Environment {
                    sender,
                    errors: &self.pending_error,
                    main_tasks: self.main_tasks.shared_sender(),
                    gpu: &self.gpu,
                };
                let native = NativeNode::create(&kind, &environment)?;
                if id.is_some() {
                    native.view.set_identifier(id.as_deref())?;
                }
                if !enabled {
                    native.set_enabled(false)?;
                }
                if help.is_some() {
                    native.view.set_tool_tip(help.as_deref())?;
                }
                native.title_bar_interactive.set(title_bar_interactive);
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
        if parent.kind() == ControlKind::ListView {
            return self.attach_rows(parent, parent_id, &children);
        }
        let elements = children
            .iter()
            .map(|(_, native)| native.view.as_element())
            .collect::<Vec<_>>();
        parent.set_children(&elements)?;
        self.attach_contents(&children)
    }

    /// Attaches the children of views that host their own children.
    fn attach_contents(&self, children: &[(NodeId, Rc<NativeNode>)]) -> Result<(), AppError> {
        for (child, native) in children {
            if let Some(control) = native.control()
                && matches!(
                    control.kind(),
                    ControlKind::Canvas
                        | ControlKind::ScrollViewer
                        | ControlKind::TabView
                        | ControlKind::ListView
                )
            {
                self.attach_children(control, *child)?;
            }
        }
        Ok(())
    }

    /// Fills a list with section headers and rows, or a table with rows of cells.
    fn attach_rows(
        &self,
        list: &Control,
        list_id: NodeId,
        children: &[(NodeId, Rc<NativeNode>)],
    ) -> Result<(), AppError> {
        let kind = self.kind(list_id);
        let mut cells = children.iter().map(|(_, native)| native.view.as_element());
        let mut entries = Vec::new();
        match &kind {
            RenderNode::View {
                kind: ViewKind::List { sections, .. },
                ..
            } => {
                for section in sections {
                    if let Some(title) = &section.title {
                        entries.push(ListEntry::Header(title));
                    }
                    for id in &section.ids {
                        let cells = cells.next().into_iter().collect();
                        entries.push(ListEntry::Row { id, cells });
                    }
                }
            }
            RenderNode::View {
                kind: ViewKind::Table { columns, rows, .. },
                ..
            } => {
                for id in rows {
                    let cells = cells.by_ref().take(columns.len().max(1)).collect();
                    entries.push(ListEntry::Row { id, cells });
                }
            }
            _ => return Err(AppError::InvalidTree),
        }
        list.set_list_entries(&entries)?;
        self.attach_contents(children)
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
        let native = self
            .state(id)
            .native
            .as_ref()
            .expect("materialized view nodes have native views")
            .clone();
        // Containers without a declared minimum follow their content instead.
        style.minimum_size = style
            .minimum_size
            .map(|declared| native.minimum_size(&kind, declared));
        style.content_size = native.content_size(&kind, style.content_size);
        let content = self.content_snapshots(id)?;
        let (children, hosted) = match kind {
            ViewKind::ScrollView { .. }
            | ViewKind::TabView { .. }
            | ViewKind::List { .. }
            | ViewKind::Table { .. } => (Vec::new(), content),
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
                        .view
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
                    host.resize(fitted, 0.0);
                }
                if *presented && (created || !was_presented) && !popover.is_open() {
                    popover.show(&anchor.view, Self::placement(*edge))?;
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
    /// Returns whether a list's sections or a table's rows or columns changed.
    fn rows_changed(old: &ViewKind, new: &ViewKind) -> bool {
        match (old, new) {
            (ViewKind::List { sections: old, .. }, ViewKind::List { sections: new, .. }) => {
                old != new
            }
            (
                ViewKind::Table {
                    columns: old_columns,
                    rows: old_rows,
                    ..
                },
                ViewKind::Table {
                    columns: new_columns,
                    rows: new_rows,
                    ..
                },
            ) => old_columns != new_columns || old_rows != new_rows,
            _ => false,
        }
    }

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
                id: old_id,
                enabled: old_enabled,
                title_bar_interactive: old_interactive,
                help: old_help,
                ..
            },
            RenderNode::View {
                kind: new_kind,
                id: new_id,
                enabled: new_enabled,
                title_bar_interactive: new_interactive,
                help: new_help,
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
        if old_id != new_id {
            native.view.set_identifier(new_id.as_deref())?;
        }
        if old_enabled != new_enabled {
            native.set_enabled(*new_enabled)?;
        }
        if old_help != new_help {
            native.view.set_tool_tip(new_help.as_deref())?;
        }
        if old_interactive != new_interactive {
            native.title_bar_interactive.set(*new_interactive);
            self.layout_dirty = true;
        }
        native.apply(Some(old_kind), new_kind)
    }
}

impl RenderBackend<RenderNode> for WinUI3Backend {
    type NodeId = NodeId;

    fn create_context<'a>(&'a self, store_context: StoreContext<'a>) -> Context<'a> {
        Context::new(store_context, Some(self.main_tasks.shared_sender()))
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
        // List sections and table rows and columns are applied together with their rows.
        if let (RenderNode::View { kind: old_kind, .. }, RenderNode::View { kind: new_kind, .. }) =
            (&old, &new)
            && Self::rows_changed(old_kind, new_kind)
        {
            self.structure_dirty = true;
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
