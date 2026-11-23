//! A table of selectable rows whose cells are arbitrary views.

use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement, KeyedElement, ListFactory};
use zintl_ui::store::Store;
use zintl_ui_backend_core::{SortOrder, TextAlignment, ViewKind};
use zintl_ui_layout::{Arrangement, Axis, CrossAxisAlignment, LayoutStyle, Size};

use crate::item_key::{ByItemKey, ItemIdFn, ItemIdSource, ItemKeys};
use crate::modifiers;
use crate::{
    Attach, Context, Element, Event, EventKind, Menu, NativeView, RenderNode, ViewModifiers,
};

type SelectionAction<K> = dyn for<'a> Fn(&mut Context<'a>, Option<&K>);
type RowAction<K> = dyn for<'a> Fn(&mut Context<'a>, &K);
type SelectionCheck<K> = dyn for<'a> Fn(&mut Context<'a>, Option<&K>) -> bool;

/// A titled column of a [`Table`].
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    title: String,
    width: Option<f32>,
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
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

/// A row of a [`Table`] and the key that selection reports for it.
#[derive(Clone)]
pub struct TableRow<K = String> {
    key: K,
    cells: ListFactory<RenderNode>,
    menu: Option<Menu>,
}

impl<K> TableRow<K> {
    /// Creates a row with one cell per column, in column order.
    ///
    /// Each cell must render exactly one view, such as a `TextField` or a stack.
    pub fn new(key: K, cells: ListFactory<RenderNode>) -> Self {
        Self {
            key,
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
/// Row keys must be unique. Rows are keyed by their item ID, from
/// [`ItemKey`](crate::ItemKey) or [`Table::item_id`], so their cell views keep
/// state when rows move. A selection set through the
/// bound Store scrolls its row into view; a key that no row has shows nothing
/// selected.
#[derive(Clone)]
pub struct Table<K: 'static = String, I = ByItemKey> {
    columns: Vec<TableColumn>,
    rows: Vec<TableRow<K>>,
    striped: bool,
    selection: Option<Store<Option<K>>>,
    action: Option<Rc<SelectionAction<K>>>,
    activate: Option<Rc<RowAction<K>>>,
    delete: Option<Rc<RowAction<K>>>,
    should_select: Option<Rc<SelectionCheck<K>>>,
    sort: Option<Store<Option<SortOrder>>>,
    fit_columns: bool,
    empty_content: Option<ElementFactory<RenderNode>>,
    summary: Option<ListFactory<RenderNode>>,
    item_id: I,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl<K: 'static> Table<K> {
    pub fn new(
        columns: impl IntoIterator<Item = TableColumn>,
        rows: impl IntoIterator<Item = TableRow<K>>,
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
            summary: None,
            item_id: ByItemKey,
            layout: LayoutStyle::fill(Size::new(0.0, 0.0)),
            modifiers: ViewModifiers::default(),
        }
    }
}

impl<K: 'static, I> Table<K, I> {
    /// Names rows by `item_id` instead of their key's `ItemKey` implementation.
    ///
    /// Use it for keys that cannot implement `ItemKey`, such as ID types from
    /// another crate. Different keys must return different strings.
    pub fn item_id(self, item_id: impl Fn(&K) -> String + 'static) -> Table<K, ItemIdFn<K>> {
        Table {
            columns: self.columns,
            rows: self.rows,
            striped: self.striped,
            selection: self.selection,
            action: self.action,
            activate: self.activate,
            delete: self.delete,
            should_select: self.should_select,
            sort: self.sort,
            fit_columns: self.fit_columns,
            empty_content: self.empty_content,
            summary: self.summary,
            item_id: ItemIdFn::new(item_id),
            layout: self.layout,
            modifiers: self.modifiers,
        }
    }

    /// Shows `cells`, one per column, in a row that stays below the rows, such as totals.
    ///
    /// The row cannot be selected, is not sorted, and stays visible while the
    /// rows scroll. Its cells follow the columns' widths and alignment.
    pub fn summary_row(mut self, cells: ListFactory<RenderNode>) -> Self {
        self.summary = Some(cells);
        self
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

    /// Reads and writes the selected row's key.
    pub fn bind(mut self, selection: Store<Option<K>>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Runs after the user changes the selection, once the bound Store is updated.
    pub fn on_select(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, Option<&K>) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Reads and writes the sort order the header shows.
    ///
    /// Clicking a column with a [`TableColumn::sort_key`] writes the Store;
    /// the app sorts its rows by it, and rows stay keyed by their keys.
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
        check: impl for<'a> Fn(&mut Context<'a>, Option<&K>) -> bool + 'static,
    ) -> Self {
        self.should_select = Some(Rc::new(check));
        self
    }

    /// Runs with a row's key when it is opened: double-clicked, or Return pressed while it is selected.
    pub fn on_activate(mut self, action: impl for<'a> Fn(&mut Context<'a>, &K) + 'static) -> Self {
        self.activate = Some(Rc::new(action));
        self
    }

    /// Runs with the selected row's key when Delete or Forward Delete is pressed in the table.
    ///
    /// The app decides what to do, for example ask before deleting.
    pub fn on_delete(mut self, action: impl for<'a> Fn(&mut Context<'a>, &K) + 'static) -> Self {
        self.delete = Some(Rc::new(action));
        self
    }

    /// Returns `cells` padded with empty cells or cut to the column count.
    fn padded(&self, cells: &ListFactory<RenderNode>) -> ListFactory<RenderNode> {
        // Each cell needs exactly one native view, so blanks are empty containers.
        let blank = || {
            ElementFactory::new(crate::Container::new(
                LayoutStyle::leaf(Size::new(0.0, 0.0)),
                Vec::new(),
            ))
        };
        let mut cells = cells.clone();
        cells.resize_with(self.columns.len().max(1), blank);
        cells
    }

    /// Returns the cells of `row`, padded with empty cells or cut to the column count.
    fn cells(&self, row: &TableRow<K>) -> impl Iterator<Item = ElementFactory<RenderNode>> {
        let mut cells = self.padded(&row.cells);
        // The table shows the menu of the clicked row's first cell, so it carries the row's menu.
        if let Some(menu) = row.menu.clone() {
            let first = cells.remove(0);
            let wrapper =
                crate::Container::new(LayoutStyle::stack(Axis::Vertical, 0.0), vec![first])
                    .context_menu(menu);
            cells.insert(0, ElementFactory::new(wrapper));
        }
        cells.into_iter()
    }
}

impl<K: Clone + PartialEq + 'static, I: ItemIdSource<K>> NativeView for Table<K, I> {
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

impl<K: Clone + PartialEq + 'static, I: ItemIdSource<K>> Table<K, I> {
    /// Returns the native table laid out with `layout`.
    fn table_element(&self, cx: &mut Context<'_>, layout: LayoutStyle) -> Element<RenderNode> {
        let keys = ItemKeys::new(self.rows.iter().map(|row| &row.key), &self.item_id);
        let cells = self
            .rows
            .iter()
            .zip(keys.ids())
            .flat_map(|(row, id)| {
                self.cells(row)
                    .enumerate()
                    .map(|(column, cell)| cell.key(format!("{id}/{column}")).into_element())
                    .collect::<Vec<_>>()
            })
            // Summary cells follow the rows, keyed apart from any row.
            .chain(self.summary.iter().flat_map(|summary| {
                self.padded(summary)
                    .into_iter()
                    .enumerate()
                    .map(|(column, cell)| cell.key(format!("\0summary/{column}")).into_element())
                    .collect::<Vec<_>>()
            }))
            .collect::<Vec<_>>();
        let selected = self.selection.and_then(|store| {
            let key = cx.get(store).as_ref()?;
            keys.id(key).map(str::to_owned)
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
                rows: keys.ids().map(str::to_owned).collect(),
                selected,
                follows_selection: self.selection.is_some() || self.should_select.is_some(),
                sort: self.sort.and_then(|store| cx.get(store).clone()),
                fit_columns: self.fit_columns,
                striped: self.striped,
                summary: self.summary.is_some(),
            },
            layout,
            // The ID names the table itself, not a container around it.
            id: self.modifiers.id().map(|id| id.as_str().to_owned()),
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
        .with_children(cells);
        let element = match self.activate.clone() {
            Some(activate) => {
                let keys = keys.clone();
                element.on_event(EventKind::RowActivated, move |cx, event| {
                    if let Event::RowActivated { id } = event
                        && let Some(key) = keys.key(&id)
                    {
                        activate(cx, key);
                    }
                })
            }
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
            Some(delete) => {
                let keys = keys.clone();
                element.on_event(EventKind::RowDeleteRequested, move |cx, event| {
                    if let Event::RowDeleteRequested { id } = event
                        && let Some(key) = keys.key(&id)
                    {
                        delete(cx, key);
                    }
                })
            }
            None => element,
        };
        let store = self.selection;
        let action = self.action.clone();
        let check = self.should_select.clone();
        if store.is_none() && action.is_none() && check.is_none() {
            return element;
        }
        element.on_event(EventKind::ListSelectionChanged, move |cx, event| {
            let Event::ListSelectionChanged { id } = event else {
                return;
            };
            // A row that is gone by the time the event arrives is ignored.
            let Some(key) = keys.selection(id.as_deref()) else {
                return;
            };
            // A rejected change leaves the Store as it is; the backend then shows it again.
            if check.as_ref().is_some_and(|check| !check(cx, key.as_ref())) {
                return;
            }
            if let Some(store) = store {
                cx.update(store, |selection| *selection = key.clone());
            }
            if let Some(action) = &action {
                action(cx, key.as_ref());
            }
        })
    }
}

impl<K: Clone + PartialEq + 'static, I: ItemIdSource<K>> zintl_ui::view::View for Table<K, I> {
    type Output = RenderNode;

    fn render(
        &self,
        cx: &mut Context<'_>,
    ) -> impl zintl_ui::element::IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
