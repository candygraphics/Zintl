//! Two panes separated by a draggable divider.

use std::ops::RangeInclusive;

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::store::{Local, Store};
use zintl_ui::view::View;
use zintl_ui_layout::{Axis, CrossAxisAlignment, Size};

use crate::graphics::{self, Graphics2D};
use crate::modifiers;
use crate::{
    Canvas2D, Canvas2DHandler, CanvasEvent, CanvasEventContext, Color, Context, Cursor, Element,
    Frame, HStack, Layout, NativeView, PaletteColor, PointerButton, PointerPhase, Redraw,
    RenderNode, VStack, ViewModifiers, list,
};

/// Two panes with a divider that the user drags to resize the first one.
///
/// The second pane takes the remaining space. Nest split views for more panes.
#[derive(Clone)]
pub struct SplitView {
    axis: Axis,
    first: ElementFactory<RenderNode>,
    second: ElementFactory<RenderNode>,
    range: RangeInclusive<f32>,
    position: Local<f32>,
    modifiers: ViewModifiers,
}

impl SplitView {
    /// Thickness of the divider's hit area in points.
    const DIVIDER: f32 = 6.0;

    /// Places the panes side by side.
    pub fn horizontal<A, B>(first: A, second: B) -> Self
    where
        A: Clone + IntoElement<Output = RenderNode> + 'static,
        B: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self::new(Axis::Horizontal, first, second)
    }

    /// Places the first pane above the second.
    pub fn vertical<A, B>(first: A, second: B) -> Self
    where
        A: Clone + IntoElement<Output = RenderNode> + 'static,
        B: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self::new(Axis::Vertical, first, second)
    }

    fn new<A, B>(axis: Axis, first: A, second: B) -> Self
    where
        A: Clone + IntoElement<Output = RenderNode> + 'static,
        B: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            axis,
            first: ElementFactory::new(first),
            second: ElementFactory::new(second),
            range: 0.0..=f32::INFINITY,
            position: Local::new(240.0),
            modifiers: ViewModifiers::default(),
        }
    }

    /// Sets the first pane's starting size along the axis.
    pub fn position(mut self, size: f32) -> Self {
        self.position.set_initial(size);
        self
    }

    /// Limits the first pane's size while dragging.
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = range;
        self
    }

    /// Reads and writes the first pane's size through `store`, for example to keep it.
    pub fn bind(mut self, store: Store<f32>) -> Self {
        self.position.bind(store);
        self
    }

    /// Wraps a pane's content.
    ///
    /// Panes may be smaller than their content, which is clipped, so wide
    /// content does not push the divider or the other pane aside.
    fn pane(content: &ElementFactory<RenderNode>) -> VStack {
        VStack::new(vec![content.clone()]).minimum_size(Size::new(0.0, 0.0))
    }

    /// Returns `size` limited to the range and never negative.
    fn clamp(&self, size: f32) -> f32 {
        size.clamp(*self.range.start(), *self.range.end()).max(0.0)
    }
}

impl View for SplitView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        let initial = self.clamp(*self.position.initial());
        self.position.set_initial(initial);
        self.position.init(cx);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        self.native_element(cx)
    }
}

impl NativeView for SplitView {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let position = self.position.store();
        let size = *cx.get(position);
        let range = self.range.clone();
        let divider =
            Canvas2D::new(Divider::new(self.axis)).on_action(move |cx, drag: &DividerDrag| {
                let (min, max) = (*range.start(), *range.end());
                cx.update(position, |size| {
                    *size = (*size + drag.delta).clamp(min, max).max(0.0);
                });
            });
        match self.axis {
            Axis::Horizontal => HStack::new(list![
                Self::pane(&self.first).width(size),
                divider.minimum_size(Size::new(Self::DIVIDER, 0.0)),
                Self::pane(&self.second).grow(),
            ])
            .spacing(0.0)
            // Panes and the divider take the split view's full height.
            .alignment(CrossAxisAlignment::Stretch)
            .fill_width()
            .grow()
            .native_element(cx),
            Axis::Vertical => VStack::new(list![
                Self::pane(&self.first).height(size),
                divider.minimum_size(Size::new(0.0, Self::DIVIDER)),
                Self::pane(&self.second).grow(),
            ])
            .spacing(0.0)
            .fill_width()
            .grow()
            .native_element(cx),
        }
    }
}

/// How far the divider was dragged along its split view's axis.
#[derive(Clone, Copy, Debug, PartialEq)]
struct DividerDrag {
    delta: f32,
}

/// Draws the divider line, follows drags, and keeps them from reaching views behind it.
#[derive(Clone)]
struct Divider {
    axis: Axis,
    /// Where the pointer pressed the divider, along the axis; set while dragging.
    ///
    /// The mounted handler keeps it across renders, so a drag survives the
    /// split view being rebuilt.
    grab: Option<f64>,
}

impl Divider {
    fn new(axis: Axis) -> Self {
        Self { axis, grab: None }
    }
}

impl Canvas2DHandler for Divider {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        let line = match self.axis {
            Axis::Horizontal => {
                graphics::Rect::new((frame.width() / 2.0).floor(), 0.0, 1.0, frame.height())
            }
            Axis::Vertical => {
                graphics::Rect::new(0.0, (frame.height() / 2.0).floor(), frame.width(), 1.0)
            }
        };
        g.clear(Color::TRANSPARENT);
        g.fill_rect(line, frame.palette.resolve(PaletteColor::Separator));
    }

    // The divider follows the pointer, so the pointer stays at the grab point
    // and each move reports its distance from there. Consuming the drag keeps
    // the window from moving when the divider sits under a transparent title bar.
    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        let CanvasEvent::Pointer(pointer) = event else {
            return false;
        };
        let along = match self.axis {
            Axis::Horizontal => pointer.location.x,
            Axis::Vertical => pointer.location.y,
        };
        match (pointer.phase, pointer.button) {
            (PointerPhase::Entered, _) => cx.set_cursor(match self.axis {
                Axis::Horizontal => Cursor::ResizeLeftRight,
                Axis::Vertical => Cursor::ResizeUpDown,
            }),
            (PointerPhase::Down, Some(PointerButton::Primary)) => self.grab = Some(along),
            (PointerPhase::Moved, Some(PointerButton::Primary)) => {
                if let Some(grab) = self.grab {
                    cx.emit(DividerDrag {
                        delta: (along - grab) as f32,
                    });
                }
            }
            (PointerPhase::Up, _) => self.grab = None,
            _ => {}
        }
        matches!(
            pointer.phase,
            PointerPhase::Down | PointerPhase::Moved | PointerPhase::Up
        ) && pointer.button.is_some()
    }

    fn reconcile(&mut self, next: Self) -> Redraw {
        // Keep the grab point of a drag in progress.
        let redraw = if self.axis == next.axis {
            Redraw::No
        } else {
            Redraw::Yes
        };
        self.axis = next.axis;
        redraw
    }
}
