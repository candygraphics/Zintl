//! A horizontal bar chart built from native views.

use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::view::View;
use zintl_ui_layout::{Axis, CrossAxisAlignment, LayoutDimension, LayoutStyle, Size};

use crate::{
    AccessibilityRole, Attach, Canvas2D, Canvas2DHandler, Color, Context, Element, Frame,
    Graphics2D, Layout, ListFactory, NativeView, PaletteColor, RenderNode, Text, TextAlignment,
    ViewModifiers, list,
};

type FormatValue = dyn Fn(f64) -> String;

/// One bar of a [`BarChart`]: a label and a value.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartBar {
    label: String,
    value: f64,
    color: Option<PaletteColor>,
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

    /// Fills this bar with `color`, such as `SystemColor::Green`, instead of the chart's color.
    pub fn color(mut self, color: impl Into<PaletteColor>) -> Self {
        self.color = Some(color.into());
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
/// value, or [`BarChart::maximum`], fills the width beside the labels;
/// negative values show no bar. Accessibility reports the chart as a group
/// and each bar with its label and formatted value.
#[derive(Clone)]
pub struct BarChart {
    bars: Vec<ChartBar>,
    format: Rc<FormatValue>,
    color: PaletteColor,
    maximum: Option<f64>,
    label_width: Option<f32>,
    value_width: Option<f32>,
    modifiers: ViewModifiers,
}

impl BarChart {
    /// The height of each bar.
    const BAR_HEIGHT: f32 = 16.0;
    /// The size of the system font the labels use.
    const FONT_SIZE: f32 = 13.0;

    pub fn new(bars: impl IntoIterator<Item = ChartBar>) -> Self {
        Self {
            bars: bars.into_iter().collect(),
            format: Rc::new(|value| format!("{value}")),
            color: PaletteColor::Accent,
            maximum: None,
            label_width: None,
            value_width: None,
            modifiers: ViewModifiers::default(),
        }
    }

    /// Formats values for the value column and accessibility, such as `|value| format!("{value:.0}")`.
    pub fn value_format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Fills bars without their own color with `color`; the default is the accent color.
    pub fn color(mut self, color: impl Into<PaletteColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the value that fills the track, such as the total of a breakdown.
    ///
    /// Larger values also fill the track. By default the largest value fills it.
    pub fn maximum(mut self, maximum: f64) -> Self {
        self.maximum = Some(maximum);
        self
    }

    /// Fixes the label column's width, so several charts line up.
    ///
    /// By default the column fits the chart's longest label.
    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = Some(width);
        self
    }

    /// Fixes the value column's width, so several charts line up.
    ///
    /// By default the column fits the chart's longest formatted value.
    pub fn value_width(mut self, width: f32) -> Self {
        self.value_width = Some(width);
        self
    }

    /// Returns each bar's length as a fraction of the track.
    pub fn fractions(&self) -> Vec<f32> {
        let maximum = self.maximum.unwrap_or_else(|| {
            self.bars
                .iter()
                .map(|bar| bar.value)
                .fold(0.0_f64, f64::max)
        });
        self.bars
            .iter()
            .map(|bar| {
                if maximum > 0.0 {
                    (bar.value.max(0.0) / maximum).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Estimates the width that shows `text` on one line in the system font.
    ///
    /// East Asian wide and full-width characters take a full em; others take
    /// an average Latin glyph's width.
    pub(crate) fn text_width(text: &str) -> f32 {
        let width: f32 = text
            .chars()
            .map(|character| {
                if Self::is_wide(character) {
                    Self::FONT_SIZE
                } else {
                    7.5
                }
            })
            .sum();
        width + 8.0
    }

    /// Returns whether `character` is East Asian wide or full-width, such as kanji or kana.
    fn is_wide(character: char) -> bool {
        matches!(
            character as u32,
            0x1100..=0x115F
                | 0x2E80..=0x303E
                | 0x3041..=0x33FF
                | 0x3400..=0x4DBF
                | 0x4E00..=0x9FFF
                | 0xA000..=0xA4CF
                | 0xAC00..=0xD7A3
                | 0xF900..=0xFAFF
                | 0xFE30..=0xFE4F
                | 0xFF00..=0xFF60
                | 0xFFE0..=0xFFE6
                | 0x1F300..=0x1F64F
                | 0x1F900..=0x1F9FF
                | 0x20000..=0x2FFFD
                | 0x30000..=0x3FFFD
        )
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

    /// Returns the label and value column widths: the fixed ones, or the widest estimate.
    fn columns(&self) -> (f32, f32) {
        let widest = |text: &dyn Fn(&ChartBar) -> String| {
            self.bars
                .iter()
                .map(|bar| Self::text_width(&text(bar)))
                .fold(0.0, f32::max)
        };
        (
            self.label_width
                .unwrap_or_else(|| widest(&|bar| bar.label.clone())),
            self.value_width
                .unwrap_or_else(|| widest(&|bar| (self.format)(bar.value))),
        )
    }
}

impl NativeView for BarChart {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let columns = self.columns();
        let rows = self
            .bars
            .iter()
            .zip(self.fractions())
            .map(|(bar, fraction)| self.row(bar, fraction, columns))
            .collect::<ListFactory<RenderNode>>();
        let mut layout = LayoutStyle::stack(Axis::Vertical, 6.0);
        layout.width = LayoutDimension::Percent(1.0);
        // The group carries the chart's ID and scopes its bars for assistive apps and UI tests.
        zintl_ui_native::View::new(layout, rows)
            .accessibility_role(AccessibilityRole::Group)
            .native_element(cx)
    }

    fn modifiers(&self) -> &ViewModifiers {
        &self.modifiers
    }

    fn modifiers_mut(&mut self) -> &mut ViewModifiers {
        &mut self.modifiers
    }
}

impl View for BarChart {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}

/// Fills a bar with one color, resolved for the appearance it is drawn in.
#[derive(Clone)]
struct BarFill {
    color: PaletteColor,
}

impl Canvas2DHandler for BarFill {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        g.clear(Color::TRANSPARENT);
        g.fill_rect(frame.bounds, frame.palette.resolve(self.color));
    }
}
