//! A table of selectable rows whose cells are arbitrary views.

use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement, KeyedElement, ListFactory};
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_backend_core::{SortOrder, TextAlignment, ViewKind};
use zintl_ui_layout::{Arrangement, Axis, CrossAxisAlignment, LayoutStyle, Size};

use crate::modifiers;
use crate::{
    Attach, Context, Element, Event, EventKind, Menu, NativeView, RenderNode, ViewModifiers,
};

type SelectionAction = dyn for<'a> Fn(&mut Context<'a>, Option<&str>);
type RowAction = dyn for<'a> Fn(&mut Context<'a>, &str);
type SelectionCheck = dyn for<'a> Fn(&mut Context<'a>, Option<&str>) -> bool;

/// A titled column of a [`Table`].
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    title: String,
    width: Option<f64>,
    alignment: TextAlignment,
    sort_key: Option<String>,
}

impl TableColumn {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: None,
            alignment: TextAlignment::Leading,
            sort_key: None,
        }
    }

    /// Lets the user sort by this column; clicking its header reports `key` to [`Table::bind_sort`].
    pub fn sort_key(mut self, key: impl Into<String>) -> Self {
        self.sort_key = Some(key.into());
        self
    }

    /// Aligns the header title, and text cells that keep the default leading alignment.
    ///
    /// Numeric columns are usually trailing-aligned.
    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets the initial rendered width in points, including the space between columns.
    ///
    /// The widths of all columns add up to the table's width. The user can
    /// still resize the column.
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
    menu: Option<Menu>,
}

impl TableRow {
    /// Creates a row with one cell per column, in column order.
    ///
    /// Each cell must render exactly one view, such as a `TextField` or a stack.
    pub fn new(id: impl Into<String>, cells: ListFactory<RenderNode>) -> Self {
        Self {
            id: id.into(),
            cells,
            menu: None,
        }
    }

    /// Shows `menu` when the row is right-clicked, without changing the selection.
    pub fn context_menu(mut self, menu: impl Into<Menu>) -> Self {
        self.menu = Some(menu.into());
        self
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
    activate: Option<Rc<RowAction>>,
    delete: Option<Rc<RowAction>>,
    should_select: Option<Rc<SelectionCheck>>,
    sort: Option<Store<Option<SortOrder>>>,
    fit_columns: bool,
    empty_content: Option<ElementFactory<RenderNode>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
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
            activate: None,
            delete: None,
            should_select: None,
            sort: None,
            fit_columns: false,
            empty_content: None,
            layout: LayoutStyle::fill(Size::new(0.0, 0.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    /// Shrinks columns proportionally whenever they are wider than the table, so none is hidden.
    ///
    /// Without it, columns that do not fit are reached by scrolling sideways.
    /// No column gets narrower than its header title.
    pub fn fit_columns(mut self, fit: bool) -> Self {
        self.fit_columns = fit;
        self
    }

    /// Shows `content` centered in the table while it has no rows, such as "No items yet".
    ///
    /// The column headers stay visible.
    pub fn empty_content<V>(mut self, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        self.empty_content = Some(ElementFactory::new(content));
        self
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

    /// Reads and writes the sort order the header shows.
    ///
    /// Clicking a column with a [`TableColumn::sort_key`] writes the Store;
    /// the app sorts its rows by it, so rows stay keyed by ID.
    pub fn bind_sort(mut self, order: Store<Option<SortOrder>>) -> Self {
        self.sort = Some(order);
        self
    }

    /// Decides whether the user may select a row, or clear the selection for `None`.
    ///
    /// It runs for clicks and keyboard navigation before the bound Store and
    /// `on_select`. Returning `false` keeps the current row selected; the app
    /// can then ask, for example about unsaved edits, and change the Store later.
    pub fn should_select(
        mut self,
        check: impl for<'a> Fn(&mut Context<'a>, Option<&str>) -> bool + 'static,
    ) -> Self {
        self.should_select = Some(Rc::new(check));
        self
    }

    /// Runs with a row's ID when it is opened: double-clicked, or Return pressed while it is selected.
    pub fn on_activate(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static,
    ) -> Self {
        self.activate = Some(Rc::new(action));
        self
    }

    /// Runs with the selected row's ID when Delete or Forward Delete is pressed in the table.
    ///
    /// The app decides what to do, for example ask before deleting.
    pub fn on_delete(mut self, action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static) -> Self {
        self.delete = Some(Rc::new(action));
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
        // The table shows the menu of the clicked row's first cell, so it carries the row's menu.
        if let Some(menu) = row.menu.clone() {
            let first = cells.remove(0);
            let wrapper = crate::View::new(LayoutStyle::stack(Axis::Vertical, 0.0), vec![first])
                .context_menu(menu);
            cells.insert(0, ElementFactory::new(wrapper));
        }
        cells.into_iter()
    }
}

impl NativeView for Table {
    modifiers!();

    fn places_id(&self) -> bool {
        true
    }

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let Some(empty) = &self.empty_content else {
            return self.table_element(cx, self.layout);
        };
        // The table and its placeholder share one cell; the placeholder is centered at its own size.
        let mut overlay = self.layout;
        overlay.arrangement = Arrangement::Overlay;
        overlay.cross_axis_alignment = CrossAxisAlignment::Center;
        let table = self.table_element(cx, LayoutStyle::fill(Size::new(0.0, 0.0)));
        let placeholder = self.rows.is_empty().then(|| empty.clone().into_element());
        Element::node(RenderNode::View {
            kind: ViewKind::Container,
            layout: overlay,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
        .with_children(std::iter::once(table).chain(placeholder))
    }
}

impl Table {
    /// Returns the native table laid out with `layout`.
    fn table_element(&self, cx: &mut Context<'_>, layout: LayoutStyle) -> Element<RenderNode> {
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
                        alignment: column.alignment,
                        sort_key: column.sort_key.clone(),
                    })
                    .collect(),
                rows: self.rows.iter().map(|row| row.id.clone()).collect(),
                selected: self.selection.and_then(|store| cx.get(store).clone()),
                follows_selection: self.selection.is_some() || self.should_select.is_some(),
                sort: self.sort.and_then(|store| cx.get(store).clone()),
                fit_columns: self.fit_columns,
                striped: self.striped,
            },
            layout,
            // The ID names the table itself, not a container around it.
            id: self.modifiers.id().map(str::to_owned),
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
        .with_children(cells);
        let element = match self.activate.clone() {
            Some(activate) => element.on_event(EventKind::RowActivated, move |cx, event| {
                if let Event::RowActivated { id } = event {
                    activate(cx, &id);
                }
            }),
            None => element,
        };
        let element = match self.sort {
            Some(store) => element.on_event(EventKind::SortChanged, move |cx, event| {
                if let Event::SortChanged { order } = event {
                    cx.update(store, |sort| *sort = Some(order));
                }
            }),
            None => element,
        };
        let element = match self.delete.clone() {
            Some(delete) => element.on_event(EventKind::RowDeleteRequested, move |cx, event| {
                if let Event::RowDeleteRequested { id } = event {
                    delete(cx, &id);
                }
            }),
            None => element,
        };
        let store = self.selection;
        let action = self.action.clone();
        let check = self.should_select.clone();
        if store.is_none() && action.is_none() && check.is_none() {
            return element;
        }
        element.on_event(EventKind::ListSelectionChanged, move |cx, event| {
            if let Event::ListSelectionChanged { id } = event {
                // A rejected change leaves the Store as it is; the backend then shows it again.
                if check
                    .as_ref()
                    .is_some_and(|check| !check(cx, id.as_deref()))
                {
                    return;
                }
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
