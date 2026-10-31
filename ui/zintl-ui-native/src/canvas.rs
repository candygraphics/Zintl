//! A view drawn by a [`Canvas2DHandler`].

use std::rc::Rc;

use zintl_ui::element::IntoElement;
use zintl_ui::view::View;
use zintl_ui_backend_core::{Canvas2DHandler, CanvasEvent, CanvasNode, RedrawPolicy, ViewKind};
use zintl_ui_layout::{LayoutDimension, LayoutStyle, Size};

use crate::attach::attachments;
use crate::{Attachments, Context, Element, Event, EventKind, NativeView, RenderNode};

type CanvasAction = dyn for<'a> Fn(&mut Context<'a>, &CanvasEvent);

/// A view that its handler draws with the platform's 2D graphics API.
///
/// Drawing runs separately from view updates, like `MTKView` and HTML
/// Canvas. Each render passes a new handler, and the mounted handler receives
/// it through [`Canvas2DHandler::reconcile`].
#[derive(Clone)]
pub struct Canvas2D<H> {
    handler: H,
    redraw: RedrawPolicy,
    focusable: bool,
    action: Option<Rc<CanvasAction>>,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl<H: Canvas2DHandler> Canvas2D<H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            redraw: RedrawPolicy::OnDemand,
            focusable: false,
            action: None,
            layout: LayoutStyle::leaf(Size::new(0.0, 0.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn redraw(mut self, policy: RedrawPolicy) -> Self {
        self.redraw = policy;
        self
    }

    /// Lets a click or the handler give the canvas keyboard focus.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Runs after the handler saw an input event, with access to Stores.
    ///
    /// Each event is queued to the view update cycle, so prefer handling
    /// high-frequency input such as hover in the handler itself.
    pub fn on_event(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, &CanvasEvent) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.layout.width = LayoutDimension::Points(size.width);
        self.layout.height = LayoutDimension::Points(size.height);
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.layout.minimum_size = size;
        self
    }

    /// Stretches the canvas across its parent's width.
    pub fn fill_width(mut self) -> Self {
        self.layout.width = LayoutDimension::Percent(1.0);
        self
    }

    /// Takes the remaining space along the parent's main axis.
    pub fn grow(mut self) -> Self {
        self.layout.grow = 1.0;
        self
    }
}

impl<H: Canvas2DHandler> NativeView for Canvas2D<H> {
    attachments!();

    fn base_element(&self, _cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::Canvas2D {
                canvas: CanvasNode::new(self.handler.clone()),
                redraw: self.redraw,
                focusable: self.focusable,
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: true,
            title_bar_interactive: false,
        });
        let Some(action) = self.action.clone() else {
            return element;
        };
        element.on_event(EventKind::Canvas, move |cx, event| {
            if let Event::Canvas(event) = event {
                action(cx, &event);
            }
        })
    }
}

impl<H: Canvas2DHandler> View for Canvas2D<H> {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
