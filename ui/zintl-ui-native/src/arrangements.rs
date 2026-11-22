//! Containers that overlay, tabulate, or fold their children.

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::view::View;
use zintl_ui_layout::{Arrangement, Axis, LayoutStyle, Size};

use crate::graphics::Graphics2D;
use crate::modifiers;
use crate::{
    AccessibilityRole, Attach, BezelStyle, Button, ButtonType, Canvas2D, Canvas2DHandler,
    ChangeRoute, Color, Container, Context, Element, Frame, HStack, Layout, ListFactory,
    NativeView, PaletteColor, RenderNode, Store, Text, VStack, ViewModifiers, list,
};
use zintl_ui::store::Local;

native_view!(ZStack, Grid);

/// Stacks its children on top of each other, later children in front.
///
/// The largest child sets the size, and every child stretches to it, like a
/// label over a 3D preview.
#[derive(Clone)]
pub struct ZStack {
    children: ListFactory<RenderNode>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl ZStack {
    pub fn new(children: ListFactory<RenderNode>) -> Self {
        let mut layout = LayoutStyle::stack(Axis::Vertical, 0.0);
        layout.arrangement = Arrangement::Overlay;
        Self {
            children,
            layout,
            modifiers: ViewModifiers::default(),
        }
    }
}

impl NativeView for ZStack {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        Container::new(self.layout, self.children.clone()).native_element(cx)
    }

    modifiers!();
}

/// Fills its frame with the alternating row color, behind a grid row.
#[derive(Clone)]
struct Stripe;

impl Canvas2DHandler for Stripe {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        g.clear(Color::TRANSPARENT);
        g.fill_rect(
            frame.bounds,
            frame.palette.resolve(PaletteColor::AlternatingRow),
        );
    }
}

/// Lays out rows of cells in aligned columns, like a table without headers.
///
/// Each column is as wide as its widest cell, and the grid has as many
/// columns as its widest row covers. Rows with fewer cells leave the last
/// columns empty.
#[derive(Clone)]
pub struct Grid {
    rows: Vec<GridRow>,
    striped: bool,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl Grid {
    pub fn new(rows: impl IntoIterator<Item = GridRow>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
            striped: false,
            layout: LayoutStyle::stack(Axis::Vertical, 4.0),
            modifiers: ViewModifiers::default(),
        }
    }

    /// Shades every other row, starting with the second.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Sets the gap between rows and between columns.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.layout.gap = spacing;
        self
    }

    /// Returns a stripe that covers `row`.
    fn stripe(row: usize) -> ElementFactory<RenderNode> {
        let mut layout = LayoutStyle::stack(Axis::Horizontal, 0.0);
        layout.grid_cell = Some(zintl_ui_layout::GridCell {
            row: row as u16,
            column: 0,
            column_span: 0,
        });
        ElementFactory::new(Container::new(layout, list![Canvas2D::new(Stripe).grow()]))
    }
}

impl NativeView for Grid {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let columns = self
            .rows
            .iter()
            .map(GridRow::width)
            .max()
            .unwrap_or(1)
            .max(1);
        let mut layout = self.layout;
        layout.arrangement = Arrangement::Grid { columns };
        // Stripes come first so the cells draw over them.
        let mut children: ListFactory<RenderNode> = Vec::new();
        if self.striped {
            children.extend((1..self.rows.len()).step_by(2).map(Self::stripe));
        }
        for row in &self.rows {
            row.push_cells(columns, &mut children);
        }
        Container::new(layout, children).native_element(cx)
    }

    modifiers!();
}

/// One row of a [`Grid`].
#[derive(Clone)]
pub struct GridRow {
    cells: ListFactory<RenderNode>,
    /// The columns each cell covers, in cell order.
    spans: Vec<u16>,
}

impl GridRow {
    /// Creates a row whose cells each cover one column.
    pub fn new(cells: ListFactory<RenderNode>) -> Self {
        let spans = vec![1; cells.len()];
        Self { cells, spans }
    }

    /// Makes the cell at `index` cover `columns` columns, such as a heading
    /// between groups of rows.
    ///
    /// An `index` past the last cell is ignored, which debug builds report.
    pub fn span(mut self, index: usize, columns: u16) -> Self {
        debug_assert!(
            index < self.spans.len(),
            "a grid row has no cell at index {index}"
        );
        if let Some(span) = self.spans.get_mut(index) {
            *span = columns.max(1);
        }
        self
    }

    /// Returns the columns the row's cells cover.
    fn width(&self) -> u16 {
        self.spans.iter().sum()
    }

    /// Adds the row's cells to `children`, filling the columns it leaves empty
    /// so the next row starts in the first column.
    fn push_cells(&self, columns: u16, children: &mut ListFactory<RenderNode>) {
        for (cell, span) in self.cells.iter().zip(&self.spans) {
            children.push(if *span > 1 {
                ElementFactory::new(GridCell::new(cell.clone(), *span))
            } else {
                cell.clone()
            });
        }
        for _ in self.width()..columns {
            children.push(ElementFactory::new(Container::new(
                LayoutStyle::leaf(Size::default()),
                Vec::new(),
            )));
        }
    }
}

/// Wraps a [`GridRow`] cell that covers several columns.
#[derive(Clone)]
struct GridCell {
    content: ElementFactory<RenderNode>,
    column_span: u16,
}

impl View for GridCell {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        // The content stretches across the columns the cell covers.
        let mut layout = LayoutStyle::stack(Axis::Vertical, 0.0);
        layout.column_span = self.column_span;
        Container::new(layout, vec![self.content.clone()]).native_element(cx)
    }
}

impl GridCell {
    fn new(content: ElementFactory<RenderNode>, column_span: u16) -> Self {
        Self {
            content,
            column_span,
        }
    }
}

/// A titled section whose content shows only while it is expanded.
#[derive(Clone)]
pub struct DisclosureGroup {
    title: String,
    content: ElementFactory<RenderNode>,
    expanded: Local<bool>,
    modifiers: ViewModifiers,
}

impl DisclosureGroup {
    pub fn new<V>(title: impl Into<String>, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            title: title.into(),
            content: ElementFactory::new(content),
            expanded: Local::new(false),
            modifiers: ViewModifiers::default(),
        }
    }

    /// Reads and writes whether the group is expanded; it starts collapsed otherwise.
    pub fn bind(mut self, expanded: Store<bool>) -> Self {
        self.expanded.bind(expanded);
        self
    }
}

impl View for DisclosureGroup {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.expanded.init(cx);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        self.native_element(cx)
    }
}

impl NativeView for DisclosureGroup {
    // The ID names the disclosure button, such as for UI tests.
    fn places_id(&self) -> bool {
        true
    }

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let expanded = self.expanded.store();
        let open = *cx.get(expanded);
        // The triangle carries the title for VoiceOver, so the visible title is not read twice.
        let mut toggle = Button::untitled()
            .bezel_style(Some(BezelStyle::Disclosure))
            .button_type(ButtonType::PushOnPushOff)
            .minimum_size(Size::new(13.0, 13.0))
            .route_state(ChangeRoute::new(Some(expanded), None))
            .accessibility_label(self.title.clone());
        if let Some(id) = self.modifiers.id() {
            toggle = toggle.id(id);
        }
        let title = Text::new(self.title.clone()).accessibility_hidden(true);
        let header = HStack::new(list![toggle, title]).spacing(4.0);
        let mut rows = list![header];
        if open {
            rows.push(self.content.clone());
        }
        VStack::new(rows)
            .spacing(6.0)
            .accessibility_label(self.title.clone())
            .accessibility_role(AccessibilityRole::Group)
            .native_element(cx)
    }

    modifiers!();
}
