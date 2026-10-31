//! Two panes separated by a draggable divider.

use std::cell::Cell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_layout::{Axis, Size};

use crate::{
    Canvas2D, Canvas2DHandler, CanvasEvent, CanvasEventContext, Color, Context, Frame, Graphics2D,
    HStack, PointerButton, PointerPhase, RenderNode, VStack, graphics, list,
};

/// Two panes with a divider that the user drags to resize the first one.
///
/// The second pane takes the remaining space. Nest split views for more panes.
#[derive(Clone)]
pub struct SplitView {
    axis: Axis,
    first: ElementFactory<RenderNode>,
    second: ElementFactory<RenderNode>,
    initial: f32,
    range: RangeInclusive<f32>,
    binding: Option<Store<f32>>,
    position: Store<f32>,
    /// Where the pointer pressed the divider, along the axis; set while dragging.
    grab: Rc<Cell<Option<f64>>>,
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
            initial: 240.0,
            range: 0.0..=f32::INFINITY,
            binding: None,
            position: Store::default(),
            grab: Rc::new(Cell::new(None)),
        }
    }

    /// Sets the first pane's starting size along the axis.
    pub fn position(mut self, size: f32) -> Self {
        self.initial = size;
        self
    }

    /// Limits the first pane's size while dragging.
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = range;
        self
    }

    /// Reads and writes the first pane's size through `store`, for example to keep it.
    pub fn bind(mut self, store: Store<f32>) -> Self {
        self.binding = Some(store);
        self
    }

    /// Moves the divider to follow a drag.
    ///
    /// The divider follows the pointer, so the pointer stays at the grab point
    /// and each move reports its distance from there.
    fn drag(&self, cx: &mut Context<'_>, event: &CanvasEvent) {
        let CanvasEvent::Pointer(pointer) = event else {
            return;
        };
        let along = match self.axis {
            Axis::Horizontal => pointer.location.x,
            Axis::Vertical => pointer.location.y,
        };
        match (pointer.phase, pointer.button) {
            (PointerPhase::Down, Some(PointerButton::Primary)) => self.grab.set(Some(along)),
            (PointerPhase::Moved, Some(PointerButton::Primary)) => {
                if let Some(grab) = self.grab.get() {
                    let delta = (along - grab) as f32;
                    let (min, max) = (*self.range.start(), *self.range.end());
                    cx.update(self.position, |size| {
                        *size = (*size + delta).clamp(min, max).max(0.0);
                    });
                }
            }
            (PointerPhase::Up, _) => self.grab.set(None),
            _ => {}
        }
    }
}

impl View for SplitView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        let initial = self.initial.clamp(*self.range.start(), *self.range.end());
        self.position = self.binding.unwrap_or_else(|| cx.store(initial));
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let size = *cx.get(self.position);
        let split = self.clone();
        let divider = Canvas2D::new(Divider { axis: self.axis })
            .on_event(move |cx, event| split.drag(cx, event));
        match self.axis {
            Axis::Horizontal => HStack::new(list![
                VStack::new(vec![self.first.clone()]).width(size),
                divider.minimum_size(Size::new(Self::DIVIDER, 0.0)),
                VStack::new(vec![self.second.clone()]).grow(),
            ])
            .spacing(0.0)
            .fill_width()
            .grow()
            .into_element(),
            Axis::Vertical => VStack::new(list![
                VStack::new(vec![self.first.clone()]).height(size),
                divider.minimum_size(Size::new(0.0, Self::DIVIDER)),
                VStack::new(vec![self.second.clone()]).grow(),
            ])
            .spacing(0.0)
            .fill_width()
            .grow()
            .into_element(),
        }
    }
}

/// Draws the divider line and keeps drags from reaching views behind it.
#[derive(Clone)]
struct Divider {
    axis: Axis,
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
        g.fill_rect(line, Color::rgba(0.5, 0.5, 0.5, 0.35));
    }

    // Consuming the drag keeps the window from moving when the divider sits
    // under a transparent title bar.
    fn event(&mut self, event: &CanvasEvent, _cx: &mut CanvasEventContext) -> bool {
        matches!(
            event,
            CanvasEvent::Pointer(pointer)
                if matches!(pointer.phase, PointerPhase::Down | PointerPhase::Moved | PointerPhase::Up)
                    && pointer.button.is_some()
        )
    }
}
