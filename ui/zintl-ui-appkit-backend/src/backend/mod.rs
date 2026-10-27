use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};

use messageloop_appkit::{Context as MessageContext, MessageLoopError, Sender};
use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
use zintl_ui::view::StoreContext;
use zintl_ui_layout::LayoutError;
use zpd_appkit::actor::{ActorId, EventRouteToken};
use zpd_appkit::runloop::{Application, ApplicationError};
use zpd_appkit::ui::{
    AlertSheet, ColorPanelSession, ContextMenu, FileDialogSheet, Popover, UiError, ViewRef,
};

mod app;
mod attachment;
mod layout;
mod native;

use app::Message;
pub use app::run_composer;
use attachment::Attachment;
use layout::{HostLayout, LayoutSnapshot};
use native::NativeNode;

use super::{
    AppKitRenderNode, Color, CommandItem, CommandModifier, CommandRole, CommandSet, MainTask,
    MainTaskSender, NodeId, NodeKind, Rect, Sidebar, ViewKind, WindowAppMenu, WindowStyle,
};

/// Inset between a popover's edge and its content.
const POPOVER_PADDING: f32 = 12.0;

#[derive(Debug)]
pub enum AppError {
    Application(ApplicationError),
    Ui(UiError),
    Layout(LayoutError),
    MessageLoop(MessageLoopError),
    NoWindow,
    InvalidTree,
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Application(error) => error.fmt(formatter),
            Self::Ui(error) => error.fmt(formatter),
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
            Self::Ui(error) => Some(error),
            Self::Layout(error) => Some(error),
            Self::MessageLoop(error) => Some(error),
            Self::NoWindow | Self::InvalidTree => None,
        }
    }
}

impl From<UiError> for AppError {
    fn from(error: UiError) -> Self {
        Self::Ui(error)
    }
}

impl From<LayoutError> for AppError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

/// Native state of a rendered Window node.
struct WindowState {
    native: ActorId,
    layout: Rc<RefCell<HostLayout>>,
    style: WindowStyle,
    applied_bounds: Option<Rect>,
    applied_full_size_content_view: Option<bool>,
    /// `None` until the sidebar declaration has been applied once.
    applied_sidebar: Option<Option<Sidebar>>,
    /// Set when the user closed the window; it is not recreated afterwards.
    closed: bool,
}

struct BackendNode<R> {
    value: Option<R>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    native: Option<Rc<NativeNode>>,
    window: Option<WindowState>,
    attachment: Option<Attachment>,
    /// The attachment declaration last applied to `attachment`.
    applied: Option<NodeKind>,
    event_route: Option<EventRouteId>,
}

impl<R> BackendNode<R> {
    fn new(value: Option<R>, event_route: Option<EventRouteId>) -> Self {
        Self {
            value,
            parent: None,
            children: Vec::new(),
            native: None,
            window: None,
            attachment: None,
            applied: None,
            event_route,
        }
    }
}

pub struct AppKitBackend<R: AppKitRenderNode> {
    nodes: Vec<Option<BackendNode<R>>>,
    retired_windows: Vec<ActorId>,
    structure_dirty: bool,
    layout_dirty: bool,
    pending_error: Rc<RefCell<Option<AppError>>>,
    main_tasks: Arc<Mutex<MainTaskRoute>>,
    perform_main: Box<MainTaskSender>,
    main_menu: Option<CommandSet>,
    opened_window: bool,
}

/// Where main-thread tasks go before and after the message loop starts.
enum MainTaskRoute {
    Pending(Vec<MainTask>),
    Connected(Box<MainTaskSender>),
}

impl<R: AppKitRenderNode> Default for AppKitBackend<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: AppKitRenderNode> AppKitBackend<R> {
    pub fn new() -> Self {
        let main_tasks = Arc::new(Mutex::new(MainTaskRoute::Pending(Vec::new())));
        let route = main_tasks.clone();
        // Tasks queued during the initial mount wait until the loop connects.
        let perform_main = Box::new(move |task: MainTask| {
            match &mut *route.lock().unwrap_or_else(PoisonError::into_inner) {
                MainTaskRoute::Pending(tasks) => {
                    tasks.push(task);
                    true
                }
                MainTaskRoute::Connected(sender) => sender(task),
            }
        });
        Self {
            nodes: vec![Some(BackendNode::new(None, None))],
            retired_windows: Vec::new(),
            structure_dirty: true,
            layout_dirty: true,
            pending_error: Rc::new(RefCell::new(None)),
            main_tasks,
            perform_main,
            main_menu: None,
            opened_window: false,
        }
    }

    fn set_main_task_sender(&mut self, sender: impl Fn(MainTask) -> bool + Send + Sync + 'static) {
        let mut route = self
            .main_tasks
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let MainTaskRoute::Pending(tasks) = &mut *route {
            for task in tasks.drain(..) {
                sender(task);
            }
        }
        *route = MainTaskRoute::Connected(Box::new(sender));
    }

    pub fn value(&self, id: NodeId) -> Option<&R> {
        self.nodes
            .get(id.0)
            .and_then(Option::as_ref)
            .and_then(|node| node.value.as_ref())
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.node(id).children
    }

    /// Returns the Composer route associated with a rendered backend node.
    pub fn event_route(&self, id: NodeId) -> Option<EventRouteId> {
        self.node(id).event_route
    }

    fn node(&self, id: NodeId) -> &BackendNode<R> {
        self.nodes
            .get(id.0)
            .and_then(Option::as_ref)
            .expect("node handle must belong to this AppKit backend")
    }

    fn node_mut(&mut self, id: NodeId) -> &mut BackendNode<R> {
        self.nodes
            .get_mut(id.0)
            .and_then(Option::as_mut)
            .expect("node handle must belong to this AppKit backend")
    }

    fn kind(&self, id: NodeId) -> NodeKind {
        self.value(id)
            .expect("render nodes have values")
            .appkit_node()
    }

    fn detach(&mut self, child: NodeId) {
        let Some(parent) = self.node(child).parent else {
            return;
        };
        self.node_mut(parent)
            .children
            .retain(|candidate| *candidate != child);
        self.node_mut(child).parent = None;
    }

    /// Records the first error, ignoring natives that AppKit already closed.
    fn record_error(&mut self, error: AppError) {
        if !matches!(error, AppError::Ui(UiError::Closed)) {
            record_error(&self.pending_error, error);
        }
    }

    fn take_error(&self) -> Result<(), AppError> {
        self.pending_error.borrow_mut().take().map_or(Ok(()), Err)
    }

    fn remove_subtree(&mut self, id: NodeId) {
        let Some(children) = self
            .nodes
            .get(id.0)
            .and_then(Option::as_ref)
            .map(|node| node.children.clone())
        else {
            return;
        };
        for child in children {
            self.remove_subtree(child);
        }
        self.detach(id);
        if let Some(window) = &self.node(id).window {
            self.retired_windows.push(window.native);
        }
        if let Some(mut node) = self.nodes.get_mut(id.0).and_then(Option::take)
            && let Some(native) = node.native.take()
            && let Err(error) = native.as_view().remove_from_superview()
        {
            self.record_error(AppError::Ui(error));
        }
    }

    /// Drops native state below a window whose native object was replaced.
    fn reset_natives(&mut self, id: NodeId) {
        let node = self.node_mut(id);
        node.native = None;
        node.window = None;
        node.attachment = None;
        node.applied = None;
        for child in self.children(id).to_vec() {
            self.reset_natives(child);
        }
    }

    fn root_children(&self) -> impl Iterator<Item = (NodeId, NodeKind)> + '_ {
        self.children(NodeId(0))
            .iter()
            .map(|id| (*id, self.kind(*id)))
    }

    /// Returns declared windows that the user has not closed.
    fn window_ids(&self) -> Vec<NodeId> {
        self.root_children()
            .filter(|(id, kind)| {
                matches!(kind, NodeKind::Window { .. })
                    && !self
                        .node(*id)
                        .window
                        .as_ref()
                        .is_some_and(|state| state.closed)
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Returns whether the app should end because its last normal window is gone.
    fn should_terminate(&self) -> bool {
        self.opened_window
            && !self.root_children().any(|(id, kind)| {
                matches!(
                    kind,
                    NodeKind::Window {
                        style: WindowStyle::Normal,
                        ..
                    }
                ) && !self
                    .node(id)
                    .window
                    .as_ref()
                    .is_some_and(|state| state.closed)
            })
    }

    fn mark_window_closed(&mut self, native: ActorId) {
        for node in self.nodes.iter_mut().flatten() {
            if let Some(state) = &mut node.window
                && state.native == native
            {
                state.closed = true;
            }
        }
    }

    fn main_menu_route(&self) -> Option<EventRouteId> {
        self.root_children()
            .find(|(_, kind)| matches!(kind, NodeKind::MainMenu(_)))
            .and_then(|(id, _)| self.node(id).event_route)
    }

    fn synchronize(
        &mut self,
        application: &Application,
        cx: &MessageContext<'_, '_, Message>,
    ) -> Result<(), AppError> {
        self.take_error()?;
        for window in self.retired_windows.drain(..) {
            cx.remove_window(window);
        }
        self.synchronize_main_menu(application, cx)?;
        let window_ids = self.window_ids();
        if window_ids.is_empty() && !self.opened_window {
            return Err(AppError::NoWindow);
        }
        let rebuild_structure = self.structure_dirty;
        for window_id in window_ids {
            self.synchronize_window(application, cx, window_id, rebuild_structure)?;
        }
        self.structure_dirty = false;
        self.layout_dirty = false;
        self.take_error()
    }

    fn synchronize_main_menu(
        &mut self,
        application: &Application,
        cx: &MessageContext<'_, '_, Message>,
    ) -> Result<(), AppError> {
        let declared = self.root_children().find_map(|(_, kind)| match kind {
            NodeKind::MainMenu(commands) => Some(commands),
            _ => None,
        });
        let commands = declared
            .or_else(R::default_main_menu)
            .unwrap_or_else(quit_commands);
        if self.main_menu.as_ref() != Some(&commands) {
            let sender = cx.sender();
            application.set_commands(&commands, move |id| {
                let _ = sender.send(Message::Command(id.into()));
            })?;
            self.main_menu = Some(commands);
        }
        Ok(())
    }

    fn synchronize_window(
        &mut self,
        application: &Application,
        cx: &MessageContext<'_, '_, Message>,
        window_id: NodeId,
        rebuild_structure: bool,
    ) -> Result<(), AppError> {
        let NodeKind::Window {
            bounds,
            title,
            full_size_content_view,
            id,
            sidebar,
            style,
        } = self.kind(window_id)
        else {
            return Ok(());
        };

        // The native class cannot change, so a new style needs a new window.
        if let Some(state) = &self.node(window_id).window
            && state.style != style
        {
            cx.remove_window(state.native);
            self.reset_natives(window_id);
        }
        let new_window = self
            .node(window_id)
            .window
            .as_ref()
            .is_none_or(|state| !cx.contains_window(state.native));
        if new_window {
            let route = self.node(window_id).event_route.map(route_token);
            let native = cx.create_styled_window(route, style)?;
            let layout = Rc::new(RefCell::new(HostLayout::default()));
            let callback_layout = layout.clone();
            let callback_error = self.pending_error.clone();
            cx.with_window(native, |window| {
                window.set_content_layout_handler(move |bounds| {
                    if let Err(error) = callback_layout.borrow().layout(bounds) {
                        record_error(&callback_error, error);
                    }
                })
            })
            .ok_or(UiError::Closed)??;
            self.node_mut(window_id).window = Some(WindowState {
                native,
                layout,
                style,
                applied_bounds: None,
                applied_full_size_content_view: None,
                applied_sidebar: None,
                closed: false,
            });
            self.opened_window = true;
        }
        let rebuild_structure = rebuild_structure || new_window;
        let state = self
            .node(window_id)
            .window
            .as_ref()
            .expect("a synchronized window has native state");
        let native_id = state.native;
        let event_route = self.node(window_id).event_route.map(route_token);
        let apply_full_size_content_view =
            state.applied_full_size_content_view != Some(full_size_content_view);
        let apply_bounds =
            apply_full_size_content_view || window_bounds_need_update(state.applied_bounds, bounds);
        let apply_sidebar = state.applied_sidebar.as_ref() != Some(&sidebar);
        cx.with_window(native_id, |window| -> Result<(), AppError> {
            window
                .actor_ref()
                .set_event_route(event_route)
                .map_err(UiError::from)?;
            if apply_full_size_content_view {
                window.set_full_size_content_view(full_size_content_view)?;
            }
            if apply_bounds {
                window.set_bounds(native_rect(bounds))?;
            }
            window.set_title(&title)?;
            window.set_identifier(id.as_deref())?;
            if apply_sidebar {
                match &sidebar {
                    Some(sidebar) => window.update_sidebar(&sidebar.native())?,
                    None => window.clear_sidebar()?,
                }
            }
            Ok(())
        })
        .ok_or(UiError::Closed)??;
        let state = self
            .node_mut(window_id)
            .window
            .as_mut()
            .expect("a synchronized window has native state");
        state.applied_bounds = Some(bounds);
        state.applied_full_size_content_view = Some(full_size_content_view);
        state.applied_sidebar = Some(sidebar);

        self.materialize_children(application, window_id)?;
        if rebuild_structure {
            cx.with_window(native_id, |window| -> Result<(), AppError> {
                self.attach_children(window.content_view()?, window_id)
            })
            .ok_or(UiError::Closed)??;
        }
        let update_layout = self.layout_dirty || rebuild_structure;
        if update_layout {
            let children = self.content_snapshots(window_id)?;
            let state = self
                .node(window_id)
                .window
                .as_ref()
                .expect("a synchronized window has native state");
            *state.layout.borrow_mut() = HostLayout {
                children,
                padding: 0.0,
            };
            cx.with_window(native_id, |window| -> Result<(), AppError> {
                let content = window.content_view()?;
                content.set_needs_layout(true)?;
                content.layout_subtree_if_needed()?;
                Ok(())
            })
            .ok_or(UiError::Closed)??;
            self.take_error()?;
        }
        if new_window {
            cx.with_window(native_id, |window| window.show())
                .ok_or(UiError::Closed)??;
        }
        // Sheets attach after the window is visible.
        self.synchronize_attachments(cx, native_id, window_id, rebuild_structure, update_layout)
    }

    fn materialize_children(
        &mut self,
        application: &Application,
        parent: NodeId,
    ) -> Result<(), AppError> {
        for child in self.children(parent).to_vec() {
            if self.node(child).native.is_none()
                && let NodeKind::View {
                    kind, id, enabled, ..
                } = self.kind(child)
            {
                let native = NativeNode::create(application, &kind)?;
                native.as_view().set_identifier(id.as_deref())?;
                if !enabled {
                    native.set_enabled(false)?;
                }
                native
                    .as_view()
                    .actor_ref()
                    .set_event_route(self.node(child).event_route.map(route_token))
                    .map_err(UiError::from)?;
                self.node_mut(child).native = Some(Rc::new(native));
            }
            self.materialize_children(application, child)?;
        }
        Ok(())
    }

    /// Adds view children to `parent`; attachments are presented separately.
    fn attach_children(&self, parent: ViewRef<'_>, parent_id: NodeId) -> Result<(), AppError> {
        for child in self.children(parent_id) {
            let Some(native) = self.node(*child).native.as_ref() else {
                continue;
            };
            native.as_view().remove_from_superview()?;
            parent.add_subview(&native.as_view())?;
            self.attach_contents(*child, native)?;
        }
        Ok(())
    }

    fn attach_contents(&self, id: NodeId, native: &NativeNode) -> Result<(), AppError> {
        match native {
            NativeNode::ScrollView(scroll, _) => self.attach_children(scroll.document_view(), id),
            NativeNode::TabView(tabs) => {
                let NodeKind::View {
                    kind: ViewKind::TabView { titles, selected },
                    ..
                } = self.kind(id)
                else {
                    return Err(AppError::InvalidTree);
                };
                let pages = self
                    .children(id)
                    .iter()
                    .filter_map(|page| Some((*page, self.node(*page).native.clone()?)))
                    .collect::<Vec<_>>();
                for (_, page) in &pages {
                    page.as_view().remove_from_superview()?;
                }
                let tabs_and_pages = pages
                    .iter()
                    .enumerate()
                    .map(|(index, (_, page))| {
                        (titles.get(index).map_or("", String::as_str), page.as_view())
                    })
                    .collect::<Vec<_>>();
                tabs.set_tabs(&tabs_and_pages)?;
                tabs.select(selected)?;
                for (page_id, page) in &pages {
                    self.attach_children(page.as_view(), *page_id)?;
                }
                Ok(())
            }
            _ => self.attach_children(native.as_view(), id),
        }
    }

    fn content_snapshots(&self, parent: NodeId) -> Result<Vec<LayoutSnapshot>, AppError> {
        self.children(parent)
            .iter()
            .filter(|child| !self.kind(**child).is_attachment())
            .map(|child| self.build_layout_snapshot(*child))
            .collect()
    }

    fn build_layout_snapshot(&self, id: NodeId) -> Result<LayoutSnapshot, AppError> {
        let NodeKind::View {
            kind,
            layout: style,
            ..
        } = self.kind(id)
        else {
            return Err(AppError::InvalidTree);
        };
        let native = self
            .node(id)
            .native
            .as_ref()
            .expect("materialized view nodes have native views")
            .clone();
        let content = self.content_snapshots(id)?;
        let (children, hosted) = match kind {
            ViewKind::ScrollView { .. } | ViewKind::TabView { .. } => (Vec::new(), content),
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
        cx: &MessageContext<'_, '_, Message>,
        window: ActorId,
        id: NodeId,
        rebuild: bool,
        relayout: bool,
    ) -> Result<(), AppError> {
        for child in self.children(id).to_vec() {
            if self.kind(child).is_attachment() {
                self.synchronize_attachment(cx, window, child, rebuild, relayout)?;
            }
            self.synchronize_attachments(cx, window, child, rebuild, relayout)?;
        }
        Ok(())
    }

    fn synchronize_attachment(
        &mut self,
        cx: &MessageContext<'_, '_, Message>,
        window: ActorId,
        id: NodeId,
        rebuild: bool,
        relayout: bool,
    ) -> Result<(), AppError> {
        let kind = self.kind(id);
        let route = self.node(id).event_route.map(route_token);
        let anchor = self
            .node(id)
            .parent
            .and_then(|parent| self.node(parent).native.clone());
        let applied = self.node(id).applied.clone();
        let was_presented = applied.as_ref().is_some_and(NodeKind::is_presented);
        // A window shows one sheet at a time; retry after the current one ends.
        if matches!(
            kind,
            NodeKind::Alert {
                presented: true,
                ..
            } | NodeKind::FileDialog {
                presented: true,
                ..
            }
        ) && !was_presented
            && cx
                .with_window(window, |window| window.has_attached_sheet())
                .unwrap_or(false)
        {
            return Ok(());
        }
        match &kind {
            NodeKind::ContextMenu { items } => {
                let Some(anchor) = anchor else {
                    return Ok(());
                };
                if self.node(id).attachment.is_none() || applied.as_ref() != Some(&kind) {
                    // Drop the old menu first so its teardown cannot clear the new one.
                    self.node_mut(id).attachment = None;
                    let menu = ContextMenu::install(anchor.as_view(), items, route)?;
                    self.node_mut(id).attachment = Some(Attachment::ContextMenu(menu));
                }
            }
            NodeKind::Popover {
                presented,
                edge,
                size,
            } => {
                let Some(anchor) = anchor else {
                    return Ok(());
                };
                let created = self.node(id).attachment.is_none();
                if created {
                    let popover = Popover::new(anchor.as_view())?;
                    let layout = Rc::new(RefCell::new(HostLayout::default()));
                    let callback_layout = layout.clone();
                    let callback_error = self.pending_error.clone();
                    popover.set_content_layout_handler(move |bounds| {
                        if let Err(error) = callback_layout.borrow().layout(bounds) {
                            record_error(&callback_error, error);
                        }
                    })?;
                    popover.set_event_route(route)?;
                    self.node_mut(id).attachment = Some(Attachment::Popover { popover, layout });
                }
                let Some(Attachment::Popover { popover, layout }) = &self.node(id).attachment
                else {
                    unreachable!("the popover attachment was created above");
                };
                if created || rebuild {
                    self.attach_children(popover.content_view(), id)?;
                }
                if created || rebuild || relayout {
                    *layout.borrow_mut() = HostLayout {
                        children: self.content_snapshots(id)?,
                        padding: POPOVER_PADDING,
                    };
                    let fitted = match size {
                        Some(size) => *size,
                        None => layout.borrow().fitting_size()?,
                    };
                    popover.set_content_size(
                        f64::from(fitted.width.max(1.0)),
                        f64::from(fitted.height.max(1.0)),
                    )?;
                    popover.content_view().set_needs_layout(true)?;
                }
                if *presented && (created || !was_presented) && !popover.is_shown() {
                    popover.show(*edge)?;
                } else if !*presented && popover.is_shown() {
                    popover.close()?;
                }
            }
            NodeKind::Alert { presented, config } => {
                if *presented && !was_presented {
                    self.node_mut(id).attachment = None;
                    let sheet = cx
                        .with_window(window, |window| AlertSheet::present(window, config, route))
                        .ok_or(UiError::Closed)??;
                    self.node_mut(id).attachment = Some(Attachment::Alert(Some(sheet)));
                } else if !*presented {
                    // Dropping a sheet that is still open dismisses it.
                    self.node_mut(id).attachment = None;
                }
            }
            NodeKind::FileDialog { presented, config } => {
                if *presented && !was_presented {
                    self.node_mut(id).attachment = None;
                    let sheet = cx
                        .with_window(window, |window| {
                            FileDialogSheet::present(window, config, route)
                        })
                        .ok_or(UiError::Closed)??;
                    self.node_mut(id).attachment = Some(Attachment::FileDialog(Some(sheet)));
                } else if !*presented {
                    self.node_mut(id).attachment = None;
                }
            }
            NodeKind::ColorPanel {
                presented,
                color,
                shows_alpha,
            } => {
                if *presented && !was_presented {
                    self.node_mut(id).attachment = None;
                    let session = cx
                        .with_window(window, |window| {
                            ColorPanelSession::present(window, *color, *shows_alpha, route)
                        })
                        .ok_or(UiError::Closed)??;
                    self.node_mut(id).attachment = Some(Attachment::ColorPanel(Some(session)));
                } else if *presented {
                    if let Some(Attachment::ColorPanel(Some(session))) = &self.node(id).attachment
                        && !colors_match(session.color()?, *color)
                    {
                        session.set_color(*color)?;
                    }
                } else {
                    self.node_mut(id).attachment = None;
                }
            }
            _ => {}
        }
        self.node_mut(id).applied = Some(kind);
        Ok(())
    }

    fn update_native(&self, id: NodeId, old: &NodeKind, new: &NodeKind) -> Result<(), AppError> {
        let Some(native) = self.node(id).native.as_ref() else {
            return Ok(());
        };
        let (
            NodeKind::View {
                kind: old_kind,
                id: old_id,
                enabled: old_enabled,
                ..
            },
            NodeKind::View {
                kind: new_kind,
                id: new_id,
                enabled: new_enabled,
                ..
            },
        ) = (old, new)
        else {
            return Ok(());
        };
        if old_id != new_id {
            native.as_view().set_identifier(new_id.as_deref())?;
        }
        if old_enabled != new_enabled {
            native.set_enabled(*new_enabled)?;
        }
        native.apply(Some(old_kind), new_kind)?;
        Ok(())
    }
}

impl NodeKind {
    /// Returns whether a presentable attachment is declared as shown.
    fn is_presented(&self) -> bool {
        matches!(
            self,
            Self::Popover {
                presented: true,
                ..
            } | Self::Alert {
                presented: true,
                ..
            } | Self::FileDialog {
                presented: true,
                ..
            } | Self::ColorPanel {
                presented: true,
                ..
            }
        )
    }
}

impl<R: AppKitRenderNode> RenderBackend<R> for AppKitBackend<R> {
    type NodeId = NodeId;

    fn create_context<'a>(&'a self, store_context: StoreContext<'a>) -> R::Context<'a> {
        R::create_context(store_context, Some(&*self.perform_main))
    }

    fn root(&self) -> Self::NodeId {
        NodeId(0)
    }

    fn create(&mut self, value: &R, event_route: Option<EventRouteId>) -> Self::NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes
            .push(Some(BackendNode::new(Some(value.clone()), event_route)));
        self.structure_dirty = true;
        self.layout_dirty = true;
        id
    }

    fn update(&mut self, id: Self::NodeId, value: &R) {
        let old = self.kind(id);
        let new = value.appkit_node();
        if let Err(error) = self.update_native(id, &old, &new) {
            self.record_error(error);
        }
        // Tab titles are applied together with their pages.
        if let (
            NodeKind::View {
                kind: ViewKind::TabView { titles: old, .. },
                ..
            },
            NodeKind::View {
                kind: ViewKind::TabView { titles: new, .. },
                ..
            },
        ) = (&old, &new)
            && old != new
        {
            self.structure_dirty = true;
        }
        self.node_mut(id).value = Some(value.clone());
        self.layout_dirty = true;
    }

    fn set_event_route(&mut self, id: Self::NodeId, event_route: Option<EventRouteId>) {
        self.node_mut(id).event_route = event_route;
        let token = event_route.map(route_token);
        let node = self.node(id);
        let result = match (&node.native, &node.attachment) {
            (Some(native), _) => native
                .as_view()
                .actor_ref()
                .set_event_route(token)
                .map_err(UiError::from),
            (None, Some(attachment)) => attachment.set_event_route(token),
            (None, None) => Ok(()),
        };
        if let Err(error) = result {
            self.record_error(AppError::Ui(error));
        }
    }

    fn insert_child(&mut self, parent: Self::NodeId, index: usize, child: Self::NodeId) {
        self.detach(child);
        let index = index.min(self.node(parent).children.len());
        self.node_mut(parent).children.insert(index, child);
        self.node_mut(child).parent = Some(parent);
        self.structure_dirty = true;
        self.layout_dirty = true;
    }

    fn remove(&mut self, id: Self::NodeId) {
        self.remove_subtree(id);
        self.structure_dirty = true;
        self.layout_dirty = true;
    }

    fn move_child(&mut self, parent: Self::NodeId, index: usize, child: Self::NodeId) {
        if self.node(child).parent == Some(parent)
            && self.node(parent).children.get(index) == Some(&child)
        {
            return;
        }
        self.insert_child(parent, index, child);
    }
}

fn route_token(route: EventRouteId) -> EventRouteToken {
    EventRouteToken::new(route.into_raw())
}

fn native_rect(rect: Rect) -> zpd_appkit::geometry::Rect {
    zpd_appkit::geometry::Rect::new(rect.x, rect.y, rect.width, rect.height)
}

fn window_bounds_need_update(applied: Option<Rect>, declared: Rect) -> bool {
    applied != Some(declared)
}

/// Compares colors after a round trip through AppKit's color conversion.
fn colors_match(first: Color, second: Color) -> bool {
    const TOLERANCE: f64 = 1.0 / 512.0;
    [
        first.red - second.red,
        first.green - second.green,
        first.blue - second.blue,
        first.alpha - second.alpha,
    ]
    .iter()
    .all(|difference| difference.abs() < TOLERANCE)
}

fn record_error(slot: &RefCell<Option<AppError>>, error: AppError) {
    let mut slot = slot.borrow_mut();
    if slot.is_none() {
        *slot = Some(error);
    }
}

fn quit_commands() -> CommandSet {
    let mut quit = CommandItem::new("Quit");
    quit.role = Some(CommandRole::Quit);
    quit.key = Some("q".into());
    quit.modifiers = vec![CommandModifier::Cmd];
    CommandSet {
        app_menu: Some(WindowAppMenu { items: vec![quit] }),
        menus: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
