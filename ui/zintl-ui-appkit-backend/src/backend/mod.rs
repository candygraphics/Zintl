use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

use messageloop_appkit::{Context as MessageContext, MessageLoopError, Sender};
use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
use zintl_ui::view::StoreContext;
use zintl_ui_layout::{LayoutError, Size};
use zpd_appkit::actor::{ActorId, EventRouteToken};
use zpd_appkit::runloop::{Application, ApplicationError};
use zpd_appkit::ui::{
    AlertSheet, ColorPanelSession, ContextMenu, FileDialogSheet, Popover, UiError, ViewRef,
};

mod app;
mod attachment;
mod canvas;
mod layout;
mod native;

use app::Message;
pub use app::run_composer;
use attachment::Attachment;
use native::NativeNode;

use crate::convert::{Native, Shared};

use super::{
    Color, CommandSet, Context, MainTask, NodeId, Rect, RenderNode, Sidebar, TitleBar, ViewKind,
    WindowStyle,
};
use zintl_ui_backend_core::layout::{HostLayout, LayoutSnapshot};
use zintl_ui_backend_core::tree::{ErrorSlot, MainTasks, NodeTree};

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
    layout: Rc<RefCell<HostLayout<NativeNode>>>,
    style: WindowStyle,
    applied_bounds: Option<Rect>,
    applied_title_bar: Option<TitleBar>,
    /// `None` until the sidebar declaration has been applied once.
    applied_sidebar: Option<Option<Sidebar>>,
    /// Set when the user closed the window; it is not recreated afterwards.
    closed: bool,
}

/// AppKit state attached to a rendered node.
#[derive(Default)]
struct NodeState {
    native: Option<Rc<NativeNode>>,
    window: Option<WindowState>,
    attachment: Option<Attachment>,
    /// The attachment declaration last applied to `attachment`.
    applied: Option<RenderNode>,
}

impl NodeState {
    fn is_closed_window(&self) -> bool {
        self.window.as_ref().is_some_and(|state| state.closed)
    }
}

pub struct AppKitBackend {
    tree: NodeTree<NodeState>,
    retired_windows: Vec<ActorId>,
    structure_dirty: bool,
    layout_dirty: bool,
    pending_error: ErrorSlot<AppError>,
    main_tasks: MainTasks,
    main_menu: Option<CommandSet>,
    /// Installed while the tree declares no main menu.
    default_main_menu: Option<CommandSet>,
    opened_window: bool,
}

impl Default for AppKitBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AppKitBackend {
    pub fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            retired_windows: Vec::new(),
            structure_dirty: true,
            layout_dirty: true,
            pending_error: ErrorSlot::default(),
            main_tasks: MainTasks::new(),
            main_menu: None,
            default_main_menu: None,
            opened_window: false,
        }
    }

    /// Sets the main menu installed while the tree declares none.
    pub fn set_default_main_menu(&mut self, commands: CommandSet) {
        self.default_main_menu = Some(commands);
    }

    fn set_main_task_sender(&mut self, sender: impl Fn(MainTask) -> bool + Send + Sync + 'static) {
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

    /// Records the first error, ignoring natives that AppKit already closed.
    fn record_error(&mut self, error: AppError) {
        if !matches!(error, AppError::Ui(UiError::Closed)) {
            self.pending_error.record(error);
        }
    }

    fn remove_subtree(&mut self, id: NodeId) {
        let mut retired = Vec::new();
        let mut errors = Vec::new();
        self.tree.remove_subtree(id, &mut |node| {
            if let Some(window) = &node.state.window {
                retired.push(window.native);
            }
            if let Some(native) = node.state.native
                && let Err(error) = native.as_view().remove_from_superview()
            {
                errors.push(error);
            }
        });
        self.retired_windows.extend(retired);
        for error in errors {
            self.record_error(AppError::Ui(error));
        }
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

    fn mark_window_closed(&mut self, native: ActorId) {
        for state in self.tree.states_mut() {
            if let Some(window) = &mut state.window
                && window.native == native
            {
                window.closed = true;
            }
        }
    }

    fn main_menu_route(&self) -> Option<EventRouteId> {
        self.tree.main_menu_route()
    }

    fn synchronize(
        &mut self,
        application: &Application,
        cx: &MessageContext<'_, '_, Message>,
    ) -> Result<(), AppError> {
        self.pending_error.take()?;
        for window in self.retired_windows.drain(..) {
            cx.remove_window(window);
        }
        self.synchronize_main_menu(application, cx)?;
        let window_ids = self.tree.window_ids(NodeState::is_closed_window);
        if window_ids.is_empty() && !self.opened_window {
            return Err(AppError::NoWindow);
        }
        let rebuild_structure = self.structure_dirty;
        for window_id in window_ids {
            self.synchronize_window(application, cx, window_id, rebuild_structure)?;
        }
        self.structure_dirty = false;
        self.layout_dirty = false;
        self.pending_error.take()
    }

    fn synchronize_main_menu(
        &mut self,
        application: &Application,
        cx: &MessageContext<'_, '_, Message>,
    ) -> Result<(), AppError> {
        let commands = self.tree.main_menu(self.default_main_menu.as_ref());
        if self.main_menu.as_ref() != Some(&commands) {
            let sender = cx.sender();
            application.set_commands(&commands.native(), move |id| {
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
        let RenderNode::Window {
            bounds,
            title,
            title_bar,
            id,
            sidebar,
            style,
        } = self.kind(window_id)
        else {
            return Ok(());
        };

        // The native class cannot change, so a new style needs a new window.
        if let Some(state) = &self.state(window_id).window
            && state.style != style
        {
            cx.remove_window(state.native);
            self.reset_natives(window_id);
        }
        let new_window = self
            .state(window_id)
            .window
            .as_ref()
            .is_none_or(|state| !cx.contains_window(state.native));
        if new_window {
            let route = self.event_route(window_id).map(route_token);
            let native = cx.create_styled_window(route, style.native())?;
            let layout = Rc::new(RefCell::new(HostLayout::default()));
            let callback_layout = layout.clone();
            let callback_error = self.pending_error.clone();
            cx.with_window(native, |window| {
                window.set_content_layout_handler(move |bounds| {
                    let size = Size::new(bounds.width as f32, bounds.height as f32);
                    if let Err(error) = callback_layout.borrow().layout(size) {
                        callback_error.record(error);
                    }
                })
            })
            .ok_or(UiError::Closed)??;
            self.state_mut(window_id).window = Some(WindowState {
                native,
                layout,
                style,
                applied_bounds: None,
                applied_title_bar: None,
                applied_sidebar: None,
                closed: false,
            });
            self.opened_window = true;
        }
        let rebuild_structure = rebuild_structure || new_window;
        let state = self
            .state(window_id)
            .window
            .as_ref()
            .expect("a synchronized window has native state");
        let native_id = state.native;
        let event_route = self.event_route(window_id).map(route_token);
        let apply_title_bar = state.applied_title_bar != Some(title_bar);
        let apply_bounds =
            apply_title_bar || window_bounds_need_update(state.applied_bounds, bounds);
        let apply_sidebar = state.applied_sidebar.as_ref() != Some(&sidebar);
        cx.with_window(native_id, |window| -> Result<(), AppError> {
            window
                .actor_ref()
                .set_event_route(event_route)
                .map_err(UiError::from)?;
            if apply_title_bar {
                window.set_title_bar(title_bar.native())?;
            }
            if apply_bounds {
                window.set_bounds(bounds.native())?;
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
            .state_mut(window_id)
            .window
            .as_mut()
            .expect("a synchronized window has native state");
        state.applied_bounds = Some(bounds);
        state.applied_title_bar = Some(title_bar);
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
                .state(window_id)
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
            self.pending_error.take()?;
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
            if self.state(child).native.is_none()
                && let RenderNode::View {
                    kind,
                    id,
                    enabled,
                    title_bar_interactive,
                    ..
                } = self.kind(child)
            {
                let native =
                    NativeNode::create(application, &kind, self.main_tasks.shared_sender())?;
                native.as_view().set_identifier(id.as_deref())?;
                if !enabled {
                    native.set_enabled(false)?;
                }
                if title_bar_interactive {
                    native.as_view().set_title_bar_interactive(true)?;
                }
                native
                    .as_view()
                    .actor_ref()
                    .set_event_route(self.event_route(child).map(route_token))
                    .map_err(UiError::from)?;
                self.state_mut(child).native = Some(Rc::new(native));
            }
            self.materialize_children(application, child)?;
        }
        Ok(())
    }

    /// Adds view children to `parent`; attachments are presented separately.
    fn attach_children(&self, parent: ViewRef<'_>, parent_id: NodeId) -> Result<(), AppError> {
        for child in self.children(parent_id) {
            let Some(native) = self.state(*child).native.as_ref() else {
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
                let RenderNode::View {
                    kind: ViewKind::TabView { titles, selected },
                    ..
                } = self.kind(id)
                else {
                    return Err(AppError::InvalidTree);
                };
                let pages = self
                    .children(id)
                    .iter()
                    .filter_map(|page| Some((*page, self.state(*page).native.clone()?)))
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
            layout: style,
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
        let route = self.event_route(id).map(route_token);
        let anchor = self
            .tree
            .node(id)
            .parent
            .and_then(|parent| self.state(parent).native.clone());
        let applied = self.state(id).applied.clone();
        let was_presented = applied.as_ref().is_some_and(RenderNode::is_presented);
        // A window shows one sheet at a time; retry after the current one ends.
        if kind.is_presented_sheet()
            && !was_presented
            && cx
                .with_window(window, |window| window.has_attached_sheet())
                .unwrap_or(false)
        {
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
                    let items = items.iter().map(Native::native).collect::<Vec<_>>();
                    let menu = ContextMenu::install(anchor.as_view(), &items, route)?;
                    self.state_mut(id).attachment = Some(Attachment::ContextMenu(menu));
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
                    let popover = Popover::new(anchor.as_view())?;
                    let layout = Rc::new(RefCell::new(HostLayout::default()));
                    let callback_layout = layout.clone();
                    let callback_error = self.pending_error.clone();
                    popover.set_content_layout_handler(move |bounds| {
                        let size = Size::new(bounds.width as f32, bounds.height as f32);
                        if let Err(error) = callback_layout.borrow().layout(size) {
                            callback_error.record(error);
                        }
                    })?;
                    popover.set_event_route(route)?;
                    self.state_mut(id).attachment = Some(Attachment::Popover { popover, layout });
                }
                let Some(Attachment::Popover { popover, layout }) = &self.state(id).attachment
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
                    popover.show(edge.native())?;
                } else if !*presented && popover.is_shown() {
                    popover.close()?;
                }
            }
            RenderNode::Alert { presented, config } => {
                if *presented && !was_presented {
                    self.state_mut(id).attachment = None;
                    let sheet = cx
                        .with_window(window, |window| {
                            AlertSheet::present(window, &config.native(), route)
                        })
                        .ok_or(UiError::Closed)??;
                    self.state_mut(id).attachment = Some(Attachment::Alert(Some(sheet)));
                } else if !*presented {
                    // Dropping a sheet that is still open dismisses it.
                    self.state_mut(id).attachment = None;
                }
            }
            RenderNode::FileDialog { presented, config } => {
                if *presented && !was_presented {
                    self.state_mut(id).attachment = None;
                    let sheet = cx
                        .with_window(window, |window| {
                            FileDialogSheet::present(window, &config.native(), route)
                        })
                        .ok_or(UiError::Closed)??;
                    self.state_mut(id).attachment = Some(Attachment::FileDialog(Some(sheet)));
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
                    let session = cx
                        .with_window(window, |window| {
                            ColorPanelSession::present(window, color.native(), *shows_alpha, route)
                        })
                        .ok_or(UiError::Closed)??;
                    self.state_mut(id).attachment = Some(Attachment::ColorPanel(Some(session)));
                } else if *presented {
                    if let Some(Attachment::ColorPanel(Some(session))) = &self.state(id).attachment
                        && !colors_match(session.color()?.shared(), *color)
                    {
                        session.set_color(color.native())?;
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

    fn update_native(
        &self,
        id: NodeId,
        old: &RenderNode,
        new: &RenderNode,
    ) -> Result<(), AppError> {
        let Some(native) = self.state(id).native.as_ref() else {
            return Ok(());
        };
        let (
            RenderNode::View {
                kind: old_kind,
                id: old_id,
                enabled: old_enabled,
                title_bar_interactive: old_interactive,
                ..
            },
            RenderNode::View {
                kind: new_kind,
                id: new_id,
                enabled: new_enabled,
                title_bar_interactive: new_interactive,
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
        if old_interactive != new_interactive {
            native
                .as_view()
                .set_title_bar_interactive(*new_interactive)?;
        }
        native.apply(Some(old_kind), new_kind)?;
        Ok(())
    }
}

impl RenderBackend<RenderNode> for AppKitBackend {
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
            self.record_error(error);
        }
        // Tab titles are applied together with their pages.
        if let (
            RenderNode::View {
                kind: ViewKind::TabView { titles: old, .. },
                ..
            },
            RenderNode::View {
                kind: ViewKind::TabView { titles: new, .. },
                ..
            },
        ) = (&old, &new)
            && old != new
        {
            self.structure_dirty = true;
        }
        self.tree.node_mut(id).value = Some(new);
        self.layout_dirty = true;
    }

    fn set_event_route(&mut self, id: Self::NodeId, event_route: Option<EventRouteId>) {
        self.tree.node_mut(id).event_route = event_route;
        let token = event_route.map(route_token);
        let state = self.state(id);
        let result = match (&state.native, &state.attachment) {
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
        self.tree.insert_child(parent, index, child);
        self.structure_dirty = true;
        self.layout_dirty = true;
    }

    fn remove(&mut self, id: Self::NodeId) {
        self.remove_subtree(id);
        self.structure_dirty = true;
        self.layout_dirty = true;
    }

    fn move_child(&mut self, parent: Self::NodeId, index: usize, child: Self::NodeId) {
        if !self.tree.is_at(parent, index, child) {
            self.insert_child(parent, index, child);
        }
    }
}

fn route_token(route: EventRouteId) -> EventRouteToken {
    EventRouteToken::new(route.into_raw())
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

#[cfg(test)]
mod tests;
