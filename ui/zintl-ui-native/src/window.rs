use std::rc::Rc;

use zintl_ui::element::{IntoElement, ListFactory};
use zintl_ui::list;
use zintl_ui::store::Store;
use zintl_ui::view::View;

use crate::menu::MenuActions;
use crate::{
    Alert, ColorPanel, Context, DocumentState, Element, EventKind, FileDialog, Rect, RenderNode,
    Sidebar, TitleBar, Toolbar, UndoState, WindowStyle,
};

type Action = dyn for<'a> Fn(&mut Context<'a>);
type CloseRequest = dyn for<'a> Fn(&mut Context<'a>) -> bool;

/// The app's own undo history for a window's Undo and Redo menu items.
///
/// While no text view is editing, Undo and Redo run these actions instead of
/// text undo; `can_undo` and `can_redo` enable the menu items.
#[derive(Clone)]
pub struct UndoHandler {
    can_undo: bool,
    can_redo: bool,
    undo: Option<Rc<Action>>,
    redo: Option<Rc<Action>>,
}

impl UndoHandler {
    pub fn new(can_undo: bool, can_redo: bool) -> Self {
        Self {
            can_undo,
            can_redo,
            undo: None,
            redo: None,
        }
    }

    pub fn on_undo(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.undo = Some(Rc::new(action));
        self
    }

    pub fn on_redo(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.redo = Some(Rc::new(action));
        self
    }
}

/// A sheet or panel presented on a window.
#[derive(Clone)]
enum WindowAttachment {
    Alert(Alert),
    FileDialog(FileDialog),
    ColorPanel(ColorPanel),
}

impl WindowAttachment {
    fn element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        match self {
            Self::Alert(alert) => alert.element(cx),
            Self::FileDialog(dialog) => dialog.element(cx),
            Self::ColorPanel(panel) => panel.element(cx),
        }
    }
}

#[derive(Clone)]
pub struct Window {
    sidebar: Option<Sidebar>,
    bounds: Rect,
    title: String,
    title_bar: TitleBar,
    style: WindowStyle,
    id: Option<String>,
    children: ListFactory<RenderNode>,
    attachments: Vec<WindowAttachment>,
    open: Option<Store<bool>>,
    on_close: Option<Rc<Action>>,
    toolbar: Option<Toolbar>,
    autosave: Option<String>,
    edited: bool,
    close_request: Option<Rc<CloseRequest>>,
    undo: Option<UndoHandler>,
    /// Counts confirmed close requests; each increase closes the window.
    closes: Store<u64>,
}

impl Window {
    pub fn new(bounds: Rect, title: impl Into<String>) -> Self {
        Self {
            sidebar: None,
            bounds,
            title: title.into(),
            title_bar: TitleBar::Standard,
            style: WindowStyle::Normal,
            id: None,
            children: Vec::new(),
            attachments: Vec::new(),
            open: None,
            on_close: None,
            toolbar: None,
            autosave: None,
            edited: false,
            close_request: None,
            undo: None,
            closes: Store::default(),
        }
    }

    pub fn sidebar(mut self, sidebar: Sidebar) -> Self {
        self.sidebar = Some(sidebar);
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn toolbar(mut self, toolbar: Toolbar) -> Self {
        self.toolbar = Some(toolbar);
        self
    }

    /// Saves the window frame under `name` and restores it when the window is created again.
    pub fn autosave(mut self, name: impl Into<String>) -> Self {
        self.autosave = Some(name.into());
        self
    }

    pub fn title_bar(mut self, title_bar: TitleBar) -> Self {
        self.title_bar = title_bar;
        self
    }

    /// Selects the native window class; changing it recreates the window.
    pub fn style(mut self, style: WindowStyle) -> Self {
        self.style = style;
        self
    }

    pub fn content<V>(mut self, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        self.children = list![content];
        self
    }

    pub fn alert(mut self, alert: Alert) -> Self {
        self.attachments.push(WindowAttachment::Alert(alert));
        self
    }

    pub fn file_dialog(mut self, dialog: FileDialog) -> Self {
        self.attachments.push(WindowAttachment::FileDialog(dialog));
        self
    }

    pub fn color_panel(mut self, panel: ColorPanel) -> Self {
        self.attachments.push(WindowAttachment::ColorPanel(panel));
        self
    }

    /// Shows the window while `open` is `true`; closing the window writes `false`.
    ///
    /// A window closed by the user is not reopened until it is declared again.
    pub fn bind_open(mut self, open: Store<bool>) -> Self {
        self.open = Some(open);
        self
    }

    /// Runs after the user closes the window.
    pub fn on_close(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.on_close = Some(Rc::new(action));
        self
    }

    /// Asks `request` before the close button or quitting closes the window.
    ///
    /// Returning `false` keeps the window open, for example to present an
    /// [`Alert`] about unsaved changes; close it later through
    /// [`Window::bind_open`]. Quitting asks every window and ends the app once
    /// the last one closes.
    pub fn on_close_request(
        mut self,
        request: impl for<'a> Fn(&mut Context<'a>) -> bool + 'static,
    ) -> Self {
        self.close_request = Some(Rc::new(request));
        self
    }

    /// Marks the window as having unsaved changes, shown in its close button.
    pub fn edited(mut self, edited: bool) -> Self {
        self.edited = edited;
        self
    }

    /// Lets Undo and Redo step through the app's history while no text is being edited.
    pub fn undo(mut self, handler: UndoHandler) -> Self {
        self.undo = Some(handler);
        self
    }
}

impl View for Window {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.closes = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        if self.open.is_some_and(|open| !*cx.get(open)) {
            return Element::fragment([]);
        }
        let attachments = self
            .attachments
            .iter()
            .map(|attachment| attachment.element(cx))
            .collect::<Vec<_>>();
        let mut actions = MenuActions::default();
        let mut element = Element::node(RenderNode::Window {
            sidebar: self
                .sidebar
                .as_ref()
                .map(|sidebar| sidebar.state(cx, &mut actions)),
            bounds: self.bounds,
            title: self.title.clone(),
            title_bar: self.title_bar,
            id: self.id.clone(),
            style: self.style,
            toolbar: self.toolbar.as_ref().map(|toolbar| toolbar.state(cx)),
            autosave: self.autosave.clone(),
            document: DocumentState {
                edited: self.edited,
                confirms_close: self.close_request.is_some(),
                close_requests: *cx.get(self.closes),
                undo: self.undo.as_ref().map(|undo| UndoState {
                    can_undo: undo.can_undo,
                    can_redo: undo.can_redo,
                }),
            },
        })
        .with_children([self.children.clone().into_element()])
        .with_children(attachments);
        if let Some(sidebar) = &self.sidebar {
            element = actions.route(sidebar.route(element));
        }
        if let Some(toolbar) = &self.toolbar {
            element = toolbar.route(element);
        }
        if let Some(request) = self.close_request.clone() {
            let closes = self.closes;
            element = element.on_event(EventKind::WindowCloseRequested, move |cx, _| {
                if request(cx) {
                    cx.update(closes, |closes| *closes += 1);
                }
            });
        }
        if let Some(undo) = &self.undo {
            if let Some(action) = undo.undo.clone() {
                element = element.on_event(EventKind::Undo, move |cx, _| action(cx));
            }
            if let Some(action) = undo.redo.clone() {
                element = element.on_event(EventKind::Redo, move |cx, _| action(cx));
            }
        }
        if self.open.is_some() || self.on_close.is_some() {
            let open = self.open;
            let action = self.on_close.clone();
            element = element.on_event(EventKind::WindowDidClose, move |cx, _| {
                if let Some(open) = open {
                    cx.update(open, |open| *open = false);
                }
                if let Some(action) = &action {
                    action(cx);
                }
            });
        }
        element
    }
}
