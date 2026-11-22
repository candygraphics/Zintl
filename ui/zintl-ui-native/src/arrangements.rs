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

native_view!(ZStack, Grid, GridCell);

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

/// Lays out children row by row in aligned columns, like a table without headers.
///
/// Each column is as wide as its widest cell. [`Grid::new`] flows cells into
/// rows in order; [`Grid::rows`] takes one [`GridRow`] per row, which
/// striping needs.
#[derive(Clone)]
pub struct Grid {
    columns: u16,
    content: GridContent,
    striped: bool,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

/// The cells of a [`Grid`]: flowing, or grouped into rows.
#[derive(Clone)]
enum GridContent {
    Cells(ListFactory<RenderNode>),
    Rows(Vec<GridRow>),
}

impl Grid {
    /// Flows `children` into rows of `columns` cells, one cell per child.
    ///
    /// A [`GridCell`] child covers several columns.
    pub fn new(columns: u16, children: ListFactory<RenderNode>) -> Self {
        Self::with_content(columns, GridContent::Cells(children))
    }

    /// Lays out one grid row per [`GridRow`], each starting in the first column.
    pub fn rows(columns: u16, rows: impl IntoIterator<Item = GridRow>) -> Self {
        Self::with_content(columns, GridContent::Rows(rows.into_iter().collect()))
    }

    fn with_content(columns: u16, content: GridContent) -> Self {
        let columns = columns.max(1);
        let mut layout = LayoutStyle::stack(Axis::Vertical, 4.0);
        layout.arrangement = Arrangement::Grid { columns };
        Self {
            columns,
            content,
            striped: false,
            layout,
            modifiers: ViewModifiers::default(),
        }
    }

    /// Shades every other row of a grid made with [`Grid::rows`].
    ///
    /// Grids made with [`Grid::new`] have no rows to shade, which debug
    /// builds report.
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
        let mut children: ListFactory<RenderNode> = Vec::new();
        match &self.content {
            GridContent::Cells(cells) => {
                debug_assert!(!self.striped, "only grids made with Grid::rows are striped");
                children.extend(cells.iter().cloned());
            }
            GridContent::Rows(rows) => {
                // Stripes come first so the cells draw over them.
                if self.striped {
                    children.extend((1..rows.len()).step_by(2).map(Self::stripe));
                }
                for row in rows {
                    row.push_cells(self.columns, &mut children);
                }
            }
        }
        Container::new(self.layout, children).native_element(cx)
    }

    modifiers!();
}

/// One row of a [`Grid::rows`] grid.
#[derive(Clone)]
pub struct GridRow {
    cells: ListFactory<RenderNode>,
    spans_row: bool,
}

impl GridRow {
    /// Creates a row with one cell per child, at most the grid's column count.
    ///
    /// Columns after the last cell stay empty.
    pub fn new(cells: ListFactory<RenderNode>) -> Self {
        Self {
            cells,
            spans_row: false,
        }
    }

    /// Creates a row of one cell that covers every column, such as a heading
    /// between groups of rows.
    pub fn spanning<V>(content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            cells: list![content],
            spans_row: true,
        }
    }

    /// Adds the row's cells to `children`, filling the columns it leaves empty
    /// so the next row starts in the first column.
    fn push_cells(&self, columns: u16, children: &mut ListFactory<RenderNode>) {
        if self.spans_row {
            children.extend(
                self.cells.iter().map(|cell| {
                    ElementFactory::new(GridCell::new(cell.clone()).column_span(columns))
                }),
            );
            return;
        }
        debug_assert!(
            self.cells.len() <= columns as usize,
            "a grid row has more cells than the grid has columns"
        );
        children.extend(self.cells.iter().cloned());
        for _ in self.cells.len()..columns as usize {
            children.push(ElementFactory::new(Container::new(
                LayoutStyle::leaf(Size::default()),
                Vec::new(),
            )));
        }
    }
}

/// A [`Grid`] child that covers several columns, such as a heading between groups of rows.
///
/// A span that does not fit the rest of its row starts the next row. In a
/// [`Grid::rows`] grid, use [`GridRow::spanning`] instead.
#[derive(Clone)]
pub struct GridCell {
    content: ElementFactory<RenderNode>,
    column_span: u16,
    modifiers: ViewModifiers,
}

impl GridCell {
    pub fn new<V>(content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            content: ElementFactory::new(content),
            column_span: 1,
            modifiers: ViewModifiers::default(),
        }
    }

    /// Covers `columns` columns, at most the grid's column count.
    pub fn column_span(mut self, columns: u16) -> Self {
        self.column_span = columns.max(1);
        self
    }
}

impl NativeView for GridCell {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        // The content stretches across the columns the cell covers.
        let mut layout = LayoutStyle::stack(Axis::Vertical, 0.0);
        layout.column_span = self.column_span;
        Container::new(layout, vec![self.content.clone()]).native_element(cx)
    }

    modifiers!();
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
