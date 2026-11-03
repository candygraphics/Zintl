//! A table of selectable rows whose cells are arbitrary views.

use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement, KeyedElement, ListFactory};
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_backend_core::ViewKind;
use zintl_ui_layout::{LayoutStyle, Size};

use crate::attach::attachments;
use crate::{Attachments, Context, Element, Event, EventKind, NativeView, RenderNode};

type SelectionAction = dyn for<'a> Fn(&mut Context<'a>, Option<&str>);

/// A titled column of a [`Table`].
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    title: String,
    width: Option<f64>,
}

impl TableColumn {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: None,
        }
    }

    /// Sets the initial width in points; the user can still resize the column.
    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }
}

/// A row of a [`Table`] and the ID that selection reports for it.
#[derive(Clone)]
pub struct TableRow {
    id: String,
    cells: ListFactory<RenderNode>,
}

impl TableRow {
    /// Creates a row with one cell per column, in column order.
    ///
    /// Each cell must render exactly one view, such as a `TextField` or a stack.
    pub fn new(id: impl Into<String>, cells: ListFactory<RenderNode>) -> Self {
        Self {
            id: id.into(),
            cells,
        }
    }
}

/// A native table with column headers, striped rows, and single selection.
///
/// Row IDs must be unique. Rows are keyed by ID, so their cell views keep
/// state when rows move. A selection set through the bound Store scrolls its
/// row into view.
#[derive(Clone)]
pub struct Table {
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    striped: bool,
    selection: Option<Store<Option<String>>>,
    action: Option<Rc<SelectionAction>>,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl Table {
    pub fn new(
        columns: impl IntoIterator<Item = TableColumn>,
        rows: impl IntoIterator<Item = TableRow>,
    ) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: rows.into_iter().collect(),
            striped: true,
            selection: None,
            action: None,
            layout: LayoutStyle::fill(Size::new(0.0, 0.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    /// Shades every other row; tables are striped by default.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Reads and writes the selected row ID.
    pub fn bind(mut self, selection: Store<Option<String>>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Runs after the user changes the selection, once the bound Store is updated.
    pub fn on_select(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, Option<&str>) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.layout.minimum_size = size;
        self
    }

    /// Returns the cells of `row`, padded with empty cells or cut to the column count.
    fn cells(&self, row: &TableRow) -> impl Iterator<Item = ElementFactory<RenderNode>> {
        // Each cell needs exactly one native view, so blanks are empty containers.
        let blank = || {
            ElementFactory::new(crate::View::new(
                LayoutStyle::leaf(Size::new(0.0, 0.0)),
                Vec::new(),
            ))
        };
        let mut cells = row.cells.clone();
        cells.resize_with(self.columns.len().max(1), blank);
        cells.into_iter()
    }
}

impl NativeView for Table {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let cells = self.rows.iter().flat_map(|row| {
            self.cells(row)
                .enumerate()
                .map(|(column, cell)| cell.key(format!("{}/{column}", row.id)).into_element())
                .collect::<Vec<_>>()
        });
        let element = Element::node(RenderNode::View {
            kind: ViewKind::Table {
                columns: self
                    .columns
                    .iter()
                    .map(|column| zintl_ui_backend_core::TableColumn {
                        title: column.title.clone(),
                        width: column.width,
                    })
                    .collect(),
                rows: self.rows.iter().map(|row| row.id.clone()).collect(),
                selected: self.selection.and_then(|store| cx.get(store).clone()),
                striped: self.striped,
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: true,
            title_bar_interactive: false,
            help: None,
        })
        .with_children(cells);
        let store = self.selection;
        let action = self.action.clone();
        if store.is_none() && action.is_none() {
            return element;
        }
        element.on_event(EventKind::ListSelectionChanged, move |cx, event| {
            if let Event::ListSelectionChanged { id } = event {
                if let Some(store) = store {
                    cx.update(store, |selection| *selection = id.clone());
                }
                if let Some(action) = &action {
                    action(cx, id.as_deref());
                }
            }
        })
    }
}

impl View for Table {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
