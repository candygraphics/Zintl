use std::rc::Rc;

use zintl_ui::element::{IntoElement, ListFactory};
use zintl_ui::list;
use zintl_ui::store::Store;
use zintl_ui::view::View;

use crate::{
    Alert, ColorPanel, Context, Element, EventKind, FileDialog, Rect, RenderNode, Sidebar,
    WindowStyle,
};

type Action = dyn for<'a> Fn(&mut Context<'a>);

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
    full_size_content_view: bool,
    style: WindowStyle,
    id: Option<String>,
    children: ListFactory<RenderNode>,
    attachments: Vec<WindowAttachment>,
    open: Option<Store<bool>>,
    on_close: Option<Rc<Action>>,
}

impl Window {
    pub fn new(bounds: Rect, title: impl Into<String>) -> Self {
        Self {
            sidebar: None,
            bounds,
            title: title.into(),
            full_size_content_view: false,
            style: WindowStyle::Normal,
            id: None,
            children: Vec::new(),
            attachments: Vec::new(),
            open: None,
            on_close: None,
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

    pub fn full_size_content_view(mut self) -> Self {
        self.full_size_content_view = true;
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
}

impl View for Window {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        if self.open.is_some_and(|open| !*cx.get(open)) {
            return Element::fragment([]);
        }
        let attachments = self
            .attachments
            .iter()
            .map(|attachment| attachment.element(cx))
            .collect::<Vec<_>>();
        let mut element = Element::node(RenderNode::NSWindow {
            sidebar: self.sidebar.as_ref().map(|sidebar| sidebar.state(cx)),
            bounds: self.bounds,
            title: self.title.clone(),
            full_size_content_view: self.full_size_content_view,
            id: self.id.clone(),
            style: self.style,
        })
        .with_children([self.children.clone().into_element()])
        .with_children(attachments);
        if let Some(sidebar) = &self.sidebar {
            element = sidebar.route(element);
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
