//! Views that show status without taking input.

use zintl_ui::element::IntoElement;
use zintl_ui::view::View;
use zintl_ui_backend_core::ViewKind;
use zintl_ui_layout::{Axis, LayoutDimension, LayoutStyle, Size};

use crate::attach::attachments;
use crate::{Attachments, Context, Element, NativeView, RenderNode};

/// Shows that work is in progress, as a spinner or a bar.
#[derive(Clone)]
pub struct ProgressIndicator {
    value: Option<f64>,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl ProgressIndicator {
    /// A spinner for work of unknown length.
    pub fn spinning() -> Self {
        Self {
            value: None,
            layout: LayoutStyle::leaf(Size::new(16.0, 16.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    /// A bar filled to `value` in `0.0..=1.0`.
    pub fn bar(value: f64) -> Self {
        let mut layout = LayoutStyle::leaf(Size::new(80.0, 20.0));
        layout.width = LayoutDimension::Percent(1.0);
        Self {
            value: Some(value),
            layout,
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.layout.minimum_size = Some(size);
        self
    }
}

impl NativeView for ProgressIndicator {
    attachments!();

    fn base_element(&self, _cx: &mut Context<'_>) -> Element<RenderNode> {
        Element::node(RenderNode::View {
            kind: ViewKind::ProgressIndicator { value: self.value },
            layout: self.layout,
            id: self.id.clone(),
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
    }
}

impl View for ProgressIndicator {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}

/// A thin line between groups of views.
///
/// It stretches across its stack: use [`Separator::horizontal`] in a
/// vertical stack and [`Separator::vertical`] in a horizontal one.
#[derive(Clone)]
pub struct Separator {
    layout: LayoutStyle,
    attachments: Attachments,
}

impl Separator {
    pub fn horizontal() -> Self {
        Self::along(Axis::Horizontal)
    }

    pub fn vertical() -> Self {
        Self::along(Axis::Vertical)
    }

    fn along(axis: Axis) -> Self {
        let size = match axis {
            Axis::Horizontal => Size::new(0.0, 1.0),
            Axis::Vertical => Size::new(1.0, 0.0),
        };
        let mut layout = LayoutStyle::leaf(size);
        match axis {
            Axis::Horizontal => layout.height = LayoutDimension::Points(1.0),
            Axis::Vertical => layout.width = LayoutDimension::Points(1.0),
        }
        Self {
            layout,
            attachments: Attachments::default(),
        }
    }
}

impl NativeView for Separator {
    attachments!();

    fn base_element(&self, _cx: &mut Context<'_>) -> Element<RenderNode> {
        Element::node(RenderNode::View {
            kind: ViewKind::Separator,
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
    }
}

impl View for Separator {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
