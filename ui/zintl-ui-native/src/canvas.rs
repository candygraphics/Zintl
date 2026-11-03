//! Views drawn by a [`Canvas2DHandler`] or, with the `wgpu` feature, a `WgpuCanvasHandler`.

use std::any::Any;
use std::rc::Rc;

use zintl_ui::element::IntoElement;
use zintl_ui::view::View;
use zintl_ui_backend_core::{Canvas2DHandler, CanvasEvent, CanvasNode, RedrawPolicy, ViewKind};
use zintl_ui_layout::{LayoutDimension, LayoutStyle, Size};

use crate::attach::attachments;
use crate::{Attachments, Context, Element, Event, EventKind, NativeView, RenderNode};

type CanvasAction = dyn for<'a> Fn(&mut Context<'a>, &CanvasEvent);
/// Receives every emitted action and ignores those of other types.
type ActionHandler = dyn for<'a> Fn(&mut Context<'a>, &dyn Any);

/// Declares a canvas view over a handler trait; the views differ only in how they draw.
macro_rules! canvas_view {
    (
        $(#[$meta:meta])*
        $name:ident: $handler:path => $node:path, $kind:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name<H> {
            handler: H,
            redraw: RedrawPolicy,
            focusable: bool,
            action: Option<Rc<CanvasAction>>,
            action_handlers: Vec<Rc<ActionHandler>>,
            layout: LayoutStyle,
            id: Option<String>,
            attachments: Attachments,
        }

        impl<H: $handler> $name<H> {
            pub fn new(handler: H) -> Self {
                Self {
                    handler,
                    redraw: RedrawPolicy::OnDemand,
                    focusable: false,
                    action: None,
                    action_handlers: Vec::new(),
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

            /// Runs for each action of type `A` that the handler emits with
            /// [`CanvasEventContext::emit`](crate::CanvasEventContext::emit).
            ///
            /// Actions of other types are ignored, so several handlers can
            /// each take one type.
            pub fn on_action<A: 'static>(
                mut self,
                action: impl for<'a> Fn(&mut Context<'a>, &A) + 'static,
            ) -> Self {
                self.action_handlers.push(Rc::new(move |cx, emitted: &dyn Any| {
                    if let Some(emitted) = emitted.downcast_ref::<A>() {
                        action(cx, emitted);
                    }
                }));
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

            /// As a window's content, extends under its title bar instead of starting below it.
            ///
            /// Use [`Frame::safe_area`](crate::Frame::safe_area) to keep drawing clear of it.
            pub fn ignores_safe_area(mut self) -> Self {
                self.layout.ignores_safe_area = true;
                self
            }
        }

        impl<H: $handler> NativeView for $name<H> {
            attachments!();

            fn base_element(&self, _cx: &mut Context<'_>) -> Element<RenderNode> {
                let mut element = Element::node(RenderNode::View {
                    kind: ViewKind::$kind {
                        canvas: $node(self.handler.clone()),
                        redraw: self.redraw,
                        focusable: self.focusable,
                        reports_input: self.action.is_some(),
                    },
                    layout: self.layout,
                    id: self.id.clone(),
                    enabled: true,
                    title_bar_interactive: false,
                    help: None,
                });
                if let Some(action) = self.action.clone() {
                    element = element.on_event(EventKind::Canvas, move |cx, event| {
                        if let Event::Canvas(event) = event {
                            action(cx, &event);
                        }
                    });
                }
                if !self.action_handlers.is_empty() {
                    let handlers = self.action_handlers.clone();
                    element = element.on_event(EventKind::CanvasActions, move |cx, event| {
                        let Event::CanvasActions { canvas } = event else {
                            return;
                        };
                        for action in canvas.take_actions() {
                            for handler in &handlers {
                                handler(cx, action.as_ref());
                            }
                        }
                    });
                }
                element
            }
        }

        impl<H: $handler> View for $name<H> {
            type Output = RenderNode;

            fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
                self.native_element(cx)
            }
        }
    };
}

canvas_view! {
    /// A view that its handler draws with the platform's 2D graphics API.
    ///
    /// Drawing runs separately from view updates, like `MTKView` and HTML
    /// Canvas. Each render passes a new handler, and the mounted handler receives
    /// it through [`Canvas2DHandler::reconcile`].
    Canvas2D: Canvas2DHandler => CanvasNode::new, Canvas2D
}

#[cfg(feature = "wgpu")]
canvas_view! {
    /// A view that its handler renders with wgpu.
    ///
    /// It redraws, takes input, and reconciles like [`Canvas2D`]; the handler
    /// renders into a GPU texture instead of drawing 2D commands. Each render
    /// passes a new handler, and the mounted handler receives it through
    /// [`WgpuCanvasHandler::reconcile`](zintl_ui_backend_core::WgpuCanvasHandler::reconcile).
    WgpuCanvas: zintl_ui_backend_core::WgpuCanvasHandler => CanvasNode::wgpu, WgpuCanvas
}
