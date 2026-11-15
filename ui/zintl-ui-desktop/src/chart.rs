//! A horizontal bar chart built from native views.

use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::view::View;
use zintl_ui_layout::{Axis, CrossAxisAlignment, LayoutDimension, LayoutStyle, Size};

use crate::{
    AccessibilityRole, Attach, Attachments, Canvas2D, Canvas2DHandler, Color, Context, Element,
    Frame, Graphics2D, ListFactory, NativeView, RenderNode, Text, TextAlignment, list,
};

type FormatValue = dyn Fn(f64) -> String;

/// One bar of a [`BarChart`]: a label and a value.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartBar {
    label: String,
    value: f64,
    color: Option<Color>,
    id: Option<String>,
}

impl ChartBar {
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
            id: None,
        }
    }

    /// Fills this bar with `color` instead of the chart's color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Identifies the bar's accessibility element, such as `expenses-food`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// Horizontal bars whose lengths are proportional to their values, such as a breakdown of a total.
///
/// Each row shows a label, the bar, and the formatted value. The largest
/// value fills the width beside the labels; negative values show no bar.
/// Accessibility reports the chart as a group and each bar with its label
/// and formatted value.
#[derive(Clone)]
pub struct BarChart {
    bars: Vec<ChartBar>,
    format: Rc<FormatValue>,
    color: Color,
    id: Option<String>,
    attachments: Attachments,
}

impl BarChart {
    /// The height of each bar.
    const BAR_HEIGHT: f32 = 16.0;

    pub fn new(bars: impl IntoIterator<Item = ChartBar>) -> Self {
        Self {
            bars: bars.into_iter().collect(),
            format: Rc::new(|value| format!("{value}")),
            color: Color::rgb(0.0, 0.48, 1.0),
            id: None,
            attachments: Attachments::default(),
        }
    }

    /// Formats values for the value column and accessibility, such as `|value| format!("{value:.0}")`.
    pub fn value_format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Fills bars without their own color with `color`.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Identifies the chart's accessibility group.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Returns each bar's length as a fraction of the longest one.
    pub fn fractions(bars: &[ChartBar]) -> Vec<f32> {
        let largest = bars.iter().map(|bar| bar.value).fold(0.0_f64, f64::max);
        bars.iter()
            .map(|bar| {
                if largest > 0.0 {
                    (bar.value.max(0.0) / largest) as f32
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Estimates the width that shows `text` on one line.
    fn text_width(text: &str) -> f32 {
        text.chars().count() as f32 * 7.5 + 8.0
    }

    /// Returns a column of `width` points that shows `text` on one line, truncated if longer.
    ///
    /// Fixed columns give every row's bar the same track width, so bars share one scale.
    fn column(text: String, width: f32, alignment: TextAlignment) -> zintl_ui_native::View {
        let mut layout = LayoutStyle::stack(Axis::Vertical, 0.0);
        layout.width = LayoutDimension::Points(width);
        // The bar carries the label and value for accessibility, so the texts are hidden.
        let text = Text::new(text)
            .alignment(alignment)
            .line_limit(1)
            .fill_width()
            .accessibility_hidden();
        zintl_ui_native::View::new(layout, list![text])
    }

    /// Builds the row for `bar`, whose bar takes `fraction` of the track.
    fn row(
        &self,
        bar: &ChartBar,
        fraction: f32,
        (label_width, value_width): (f32, f32),
    ) -> ElementFactory<RenderNode> {
        let value = (self.format)(bar.value);
        let label = Self::column(bar.label.clone(), label_width, TextAlignment::Leading);
        let mut fill = Canvas2D::new(BarFill {
            color: bar.color.unwrap_or(self.color),
        })
        .grow();
        if let Some(id) = &bar.id {
            fill = fill.id(id.clone());
        }
        let fill = fill
            .accessibility_label(bar.label.clone())
            .accessibility_value(value.clone());
        let mut bar_layout = LayoutStyle::stack(Axis::Vertical, 0.0);
        bar_layout.width = LayoutDimension::Percent(fraction);
        bar_layout.height = LayoutDimension::Points(Self::BAR_HEIGHT);
        let bar_view = zintl_ui_native::View::new(bar_layout, list![fill]);
        let mut track = LayoutStyle::stack(Axis::Horizontal, 0.0);
        track.grow = 1.0;
        track.minimum_size = Some(Size::new(0.0, Self::BAR_HEIGHT));
        let track = zintl_ui_native::View::new(track, list![bar_view]);
        let value = Self::column(value, value_width, TextAlignment::Trailing);
        let mut row = LayoutStyle::stack(Axis::Horizontal, 8.0);
        row.cross_axis_alignment = CrossAxisAlignment::Center;
        row.width = LayoutDimension::Percent(1.0);
        ElementFactory::new(zintl_ui_native::View::new(row, list![label, track, value]))
    }
}

impl NativeView for BarChart {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        // Labels and values each share a column as wide as the longest one is estimated to be.
        let widest = |text: &dyn Fn(&ChartBar) -> String| {
            self.bars
                .iter()
                .map(|bar| Self::text_width(&text(bar)))
                .fold(0.0, f32::max)
        };
        let columns = (
            widest(&|bar| bar.label.clone()),
            widest(&|bar| (self.format)(bar.value)),
        );
        let rows = self
            .bars
            .iter()
            .zip(Self::fractions(&self.bars))
            .map(|(bar, fraction)| self.row(bar, fraction, columns))
            .collect::<ListFactory<RenderNode>>();
        let mut layout = LayoutStyle::stack(Axis::Vertical, 6.0);
        layout.width = LayoutDimension::Percent(1.0);
        // The group exposes the chart's ID and scopes its bars for assistive apps and UI tests.
        let view =
            zintl_ui_native::View::new(layout, rows).accessibility_role(AccessibilityRole::Group);
        match &self.id {
            Some(id) => view.id(id.clone()).native_element(cx),
            None => view.native_element(cx),
        }
    }

    fn attachments(&self) -> &Attachments {
        &self.attachments
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        &mut self.attachments
    }
}

impl View for BarChart {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}

/// Fills a bar with one color.
#[derive(Clone)]
struct BarFill {
    color: Color,
}

impl Canvas2DHandler for BarFill {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        g.clear(Color::TRANSPARENT);
        g.fill_rect(frame.bounds, self.color);
    }
}
