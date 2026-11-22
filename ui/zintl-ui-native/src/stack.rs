//! Stacks that lay out their children in a row or a column.

use std::marker::PhantomData;

use zintl_ui::element::ListFactory;
use zintl_ui_layout::{
    Axis, ChildSizing, CrossAxisAlignment, LayoutStyle, MainAxisDistribution, Size,
};

use crate::modifiers;
use crate::{Container, Context, Element, NativeView, RenderNode, ViewModifiers};

/// The direction a [`Stack`] lays out its children.
pub trait StackAxis: Clone + 'static {
    const AXIS: Axis;
}

/// Marks a [`Stack`] as a row.
#[derive(Clone)]
pub struct Horizontal;

impl StackAxis for Horizontal {
    const AXIS: Axis = Axis::Horizontal;
}

/// Marks a [`Stack`] as a column.
#[derive(Clone)]
pub struct Vertical;

impl StackAxis for Vertical {
    const AXIS: Axis = Axis::Vertical;
}

/// Lays out its children along one axis; use it as [`HStack`] or [`VStack`].
#[derive(Clone)]
pub struct Stack<A> {
    children: ListFactory<RenderNode>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
    axis: PhantomData<A>,
}

/// Lays out its children in a row, centered across it.
pub type HStack = Stack<Horizontal>;

/// Lays out its children in a column, stretched across it.
pub type VStack = Stack<Vertical>;

impl<A: StackAxis> Stack<A> {
    pub fn new(children: ListFactory<RenderNode>) -> Self {
        let mut layout = LayoutStyle::stack(A::AXIS, 8.0);
        // Rows center their children, so a field beside a taller button keeps its own height.
        if A::AXIS == Axis::Horizontal {
            layout.cross_axis_alignment = CrossAxisAlignment::Center;
        }
        Self {
            children,
            layout,
            modifiers: ViewModifiers::default(),
            axis: PhantomData,
        }
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.layout.gap = spacing;
        self
    }

    /// Distributes the space the children leave along the main axis, such as
    /// [`MainAxisDistribution::SpaceBetween`] to place the first and last
    /// children at both ends.
    pub fn distribution(mut self, distribution: MainAxisDistribution) -> Self {
        self.layout.main_axis_distribution = distribution;
        self
    }

    /// Gives every child the same share of the main axis: the same width in
    /// an `HStack` and the same height in a `VStack`.
    pub fn equal_children(mut self) -> Self {
        self.layout.child_sizing = ChildSizing::Equal;
        self
    }

    /// Positions children across the main axis.
    ///
    /// `HStack` centers its children by default and `VStack` stretches
    /// them. Filling views such as tables stretch in either case.
    pub fn alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.layout.cross_axis_alignment = alignment;
        self
    }

    /// Insets the children from every edge.
    pub fn padding(mut self, padding: f32) -> Self {
        self.layout.padding = padding;
        self
    }

    /// As a window's content, extends under its title bar with `true` instead of starting below it.
    pub fn ignores_safe_area(mut self, ignores: bool) -> Self {
        self.layout.ignores_safe_area = ignores;
        self
    }
}

impl<A: StackAxis> NativeView for Stack<A> {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        Container::new(self.layout, self.children.clone()).native_element(cx)
    }
}

impl<A: StackAxis> zintl_ui::view::View for Stack<A> {
    type Output = RenderNode;

    fn render(
        &self,
        cx: &mut Context<'_>,
    ) -> impl zintl_ui::element::IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}

/// Empty space that takes the remaining room along its stack's axis.
#[derive(Clone, Default)]
pub struct Spacer {
    modifiers: ViewModifiers,
}

impl Spacer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl NativeView for Spacer {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let mut layout = LayoutStyle::leaf(Size::new(0.0, 0.0));
        layout.grow = 1.0;
        Container::new(layout, Vec::new()).native_element(cx)
    }
}

native_view!(Spacer);
