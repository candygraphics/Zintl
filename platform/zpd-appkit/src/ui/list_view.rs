use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;

use crate::actor::{ActorRef, WindowEventKind};
use crate::geometry::Rect;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, RcIvar, Strong};

use super::UiError;
use super::view::{self, AsView, ViewActor, ViewRef};

/// The table style of a [`ListView`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListStyle {
    /// Rows are inset with rounded selection, like Notes and Mail.
    #[default]
    Inset,
    /// Rows span the full width.
    Plain,
}

impl ListStyle {
    /// Returns the `NSTableViewStyle` value.
    fn native(self) -> native::Integer {
        match self {
            Self::Inset => 2,
            Self::Plain => 4,
        }
    }
}

/// A titled column of a [`ListView`] shown as a table.
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    pub title: String,
    /// The initial width in points; `None` lets the table size it.
    pub width: Option<f64>,
}

/// A table row: a section header or a selectable row with an ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListEntry {
    Header(String),
    Row(String),
}

struct Entry {
    /// `None` for section headers.
    id: Option<String>,
    /// One view per column; a section header has one that spans the row.
    cells: Vec<Strong>,
    /// The cell wrappers' Actors under the table, so cell views' events reach the window.
    actors: Vec<ActorRef>,
    height: f64,
}

struct ListState {
    entries: RefCell<Vec<Entry>>,
    /// Programmatic changes must not be reported as user selection.
    ready: Cell<bool>,
    /// The scroll view Actor, whose event route receives selection changes.
    actor: ActorRef,
}

static LIST_STATE: RcIvar<ListState> = RcIvar::new(c"_zpdListState");

impl ListState {
    const HEADER_HEIGHT: f64 = 28.0;

    fn with<R: Default>(object: Id, operation: impl FnOnce(&Self) -> R) -> R {
        zpd_objc::abort_on_unwind(|| {
            // SAFETY: Our registered delegate owns its state until teardown clears the ivar.
            let state = unsafe { LIST_STATE.get(object) };
            state.as_deref().map(operation).unwrap_or_default()
        })
    }

    fn entry<R: Default>(&self, row: i64, read: impl FnOnce(&Entry) -> R) -> R {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.entries.borrow().get(row).map(read))
            .unwrap_or_default()
    }

    /// Drops the row layout callbacks, which capture backend layout state.
    fn release_row_callbacks(&self) {
        for cell in self.row_cells() {
            // SAFETY: Row cells are live layout views; header cells are excluded
            // because they lack the callback ivar.
            unsafe { view::release_layout_callback(cell) };
        }
    }

    /// Returns the cell wrappers of selectable rows, row by row.
    fn row_cells(&self) -> Vec<Id> {
        self.entries
            .borrow()
            .iter()
            .filter(|entry| entry.id.is_some())
            .flat_map(|entry| entry.cells.iter().map(Strong::as_ptr))
            .collect()
    }
}

unsafe extern "C" fn row_count(object: Id, _: zpd_objc::Sel, _: Id) -> i64 {
    ListState::with(object, |state| state.entries.borrow().len() as i64)
}

unsafe extern "C" fn row_view(object: Id, _: zpd_objc::Sel, table: Id, column: Id, row: i64) -> Id {
    // Group rows ask with a nil column and show their first cell across the row.
    let column = if column.is_null() {
        0
    } else {
        // SAFETY: AppKit passes the live table and one of its columns.
        unsafe {
            let columns = zpd_objc::msg_send!(table, zpd_objc::sel!("tableColumns"), () => Id);
            zpd_objc::msg_send!(columns, zpd_objc::sel!("indexOfObjectIdenticalTo:"), ((column): Id) => usize)
        }
    };
    ListState::with(object, |state| {
        state.entry(row, |entry| entry.cells.get(column).map(Strong::as_ptr))
    })
    .unwrap_or(zpd_objc::NIL)
}

unsafe extern "C" fn row_height(object: Id, _: zpd_objc::Sel, _: Id, row: i64) -> f64 {
    ListState::with(object, |state| state.entry(row, |entry| entry.height)).max(1.0)
}

unsafe extern "C" fn is_group(object: Id, _: zpd_objc::Sel, _: Id, row: i64) -> bool {
    ListState::with(object, |state| state.entry(row, |entry| entry.id.is_none()))
}

unsafe extern "C" fn should_select(object: Id, _: zpd_objc::Sel, _: Id, row: i64) -> bool {
    ListState::with(object, |state| state.entry(row, |entry| entry.id.is_some()))
}

unsafe extern "C" fn selection_changed(object: Id, _: zpd_objc::Sel, notification: Id) {
    ListState::with(object, |state| {
        if !state.ready.get() {
            return;
        }
        // SAFETY: NSTableView sends this notification with itself as its object.
        let row = unsafe {
            let table = zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id);
            zpd_objc::msg_send!(table, zpd_objc::sel!("selectedRow"), () => i64)
        };
        let id = state.entry(row, |entry| entry.id.clone());
        if let Some(tree) = state.actor.tree_handle() {
            tree.emit(&state.actor, WindowEventKind::ListSelectionChanged { id });
        }
    });
}

/// Fills the table's menu with the context menu of the clicked row's view.
///
/// `NSTableView` shows its own menu on right-click and ignores row views'
/// menus, so the clicked row's items are copied in.
unsafe extern "C" fn menu_needs_update(object: Id, _: zpd_objc::Sel, menu: Id) {
    ListState::with(object, |state| {
        // SAFETY: AppKit passes the live table menu; the scroll Actor holds the table.
        unsafe {
            zpd_objc::msg_send!(menu, zpd_objc::sel!("removeAllItems"), () => ());
            let Ok(scroll) = state.actor.with(|scroll| scroll) else {
                return;
            };
            let table = zpd_objc::msg_send!(scroll, zpd_objc::sel!("documentView"), () => Id);
            let row = zpd_objc::msg_send!(table, zpd_objc::sel!("clickedRow"), () => i64);
            let Some(cell) = state.entry(row, |entry| {
                entry
                    .id
                    .is_some()
                    .then(|| entry.cells.first().map(Strong::as_ptr))
                    .flatten()
            }) else {
                return;
            };
            let subviews = zpd_objc::msg_send!(cell, zpd_objc::sel!("subviews"), () => Id);
            let content = zpd_objc::msg_send!(subviews, zpd_objc::sel!("firstObject"), () => Id);
            if content.is_null() {
                return;
            }
            let source = zpd_objc::msg_send!(content, zpd_objc::sel!("menu"), () => Id);
            if source.is_null() {
                return;
            }
            let count = zpd_objc::msg_send!(source, zpd_objc::sel!("numberOfItems"), () => i64);
            for index in 0..count {
                let item = zpd_objc::msg_send!(source, zpd_objc::sel!("itemAtIndex:"), ((index): i64) => Id);
                // Copies keep the original target and action; the row view keeps the target alive.
                let copy = Strong::from_retained(
                    zpd_objc::msg_send!(item, zpd_objc::sel!("copy"), () => Id),
                );
                if let Some(copy) = copy {
                    zpd_objc::msg_send!(menu, zpd_objc::sel!("addItem:"), ((copy.as_ptr()): Id) => ());
                }
            }
        }
    });
}

fn release(object: Id) {
    // SAFETY: The caller passes a delegate created by `ListView::new`.
    drop(unsafe { LIST_STATE.take(object) });
}

unsafe extern "C" fn dealloc(object: Id, _: zpd_objc::Sel) {
    zpd_objc::abort_on_unwind(|| release(object));
    // SAFETY: Our NSObject subclass has released its Rust state and now invokes superclass dealloc.
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSObject"), zpd_objc::sel!("dealloc"), () => ());
    }
}

fn delegate_class() -> zpd_objc::Class {
    static CLASS: OnceLock<usize> = OnceLock::new();
    *CLASS.get_or_init(|| {
        zpd_objc::decl!(ZpdListTableDelegate: [zpd_objc::class!("NSObject")] {
            fields { _zpdListState: ptr }
            methods {
                "numberOfRowsInTableView:": "q@:@" => row_count,
                "tableView:viewForTableColumn:row:": "@@:@@q" => row_view,
                "tableView:heightOfRow:": "d@:@q" => row_height,
                "tableView:isGroupRow:": "B@:@q" => is_group,
                "tableView:shouldSelectRow:": "B@:@q" => should_select,
                "tableViewSelectionDidChange:": "v@:@" => selection_changed,
                "menuNeedsUpdate:": "v@:@" => menu_needs_update,
                "dealloc": "v@:" => dealloc,
            }
        }) as usize
    }) as zpd_objc::Class
}

/// An `NSTableView` whose cells host arbitrary views.
///
/// Without columns it is a list with one untitled column; with
/// [`ListView::set_columns`] it is a table with column headers. Each cell
/// view sits in a wrapper whose layout callback receives the cell bounds, so
/// content lays out at the width the table gives it.
pub struct ListView {
    view: ViewActor,
    table: ActorRef,
    state: Rc<ListState>,
    columns: Cell<usize>,
}

impl ListView {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        let scroll = native::alloc_init(zpd_objc::class!("NSScrollView"));
        let table = native::alloc_init(zpd_objc::class!("NSTableView"));
        let view = ViewActor::new(application, scroll.clone());
        let tree = view.actor().tree_handle().ok_or(UiError::Closed)?;
        let table_actor = tree.insert_child(view.actor(), table.clone())?;
        let state = Rc::new(ListState {
            entries: RefCell::new(Vec::new()),
            ready: Cell::new(true),
            actor: view.actor().clone(),
        });
        let delegate = native::alloc_init(delegate_class());
        // SAFETY: The registered delegate class declares the `_zpdListState` ivar.
        unsafe { LIST_STATE.set(delegate.as_ptr(), state.clone()) };
        let delegate_actor = tree.insert_child(&table_actor, delegate.clone())?;
        tree.add_teardown(&delegate_actor, release)?;
        tree.add_teardown(&table_actor, |table| {
            // SAFETY: Disconnect non-owning AppKit delegates before their Rust state is released.
            unsafe {
                zpd_objc::msg_send!(table, zpd_objc::sel!("setDelegate:"), ((zpd_objc::NIL): Id) => ());
                zpd_objc::msg_send!(table, zpd_objc::sel!("setDataSource:"), ((zpd_objc::NIL): Id) => ());
                let menu = zpd_objc::msg_send!(table, zpd_objc::sel!("menu"), () => Id);
                if !menu.is_null() {
                    zpd_objc::msg_send!(menu, zpd_objc::sel!("setDelegate:"), ((zpd_objc::NIL): Id) => ());
                }
            }
        })?;
        // SAFETY: The scroll view, table, column, and delegate are live on the main thread.
        unsafe {
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setHasVerticalScroller:"), ((true): bool) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setAutohidesScrollers:"), ((true): bool) => ());
            let column = Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSTableColumn"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithIdentifier:"), ((native::nsstring("rows").as_ptr()): Id) => Id))
                .ok_or(UiError::NativeCreationFailed)?;
            zpd_objc::msg_send!(column.as_ptr(), zpd_objc::sel!("setResizingMask:"), ((native::NS_TABLE_COLUMN_AUTORESIZING_MASK): u64) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("addTableColumn:"), ((column.as_ptr()): Id) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setHeaderView:"), ((zpd_objc::NIL): Id) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setStyle:"), ((ListStyle::default().native()): native::Integer) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setRowSizeStyle:"), ((native::NS_TABLE_VIEW_ROW_SIZE_STYLE_CUSTOM): native::Integer) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setBackgroundColor:"), ((zpd_objc::msg_send!(zpd_objc::class!("NSColor"), zpd_objc::sel!("clearColor"), () => Id)): Id) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setAllowsEmptySelection:"), ((true): bool) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setAllowsMultipleSelection:"), ((false): bool) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setDataSource:"), ((delegate.as_ptr()): Id) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setDelegate:"), ((delegate.as_ptr()): Id) => ());
            let menu = native::alloc_init(zpd_objc::class!("NSMenu"));
            zpd_objc::msg_send!(menu.as_ptr(), zpd_objc::sel!("setDelegate:"), ((delegate.as_ptr()): Id) => ());
            zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setMenu:"), ((menu.as_ptr()): Id) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDocumentView:"), ((table.as_ptr()): Id) => ());
        }
        Ok(Self {
            view,
            table: table_actor,
            state,
            columns: Cell::new(1),
        })
    }

    /// Shows `columns` with headers; an empty slice shows one untitled list column.
    ///
    /// Apply the rows again with [`ListView::set_entries`] afterwards.
    pub fn set_columns(&self, columns: &[TableColumn]) -> Result<(), UiError> {
        self.columns.set(columns.len().max(1));
        self.table.with(|table| {
            // SAFETY: The Actor holds the live NSTableView; new columns and the header are retained by it.
            unsafe {
                let existing = zpd_objc::msg_send!(table, zpd_objc::sel!("tableColumns"), () => Id);
                let existing = zpd_objc::msg_send!(existing, zpd_objc::sel!("copy"), () => Id);
                let count = zpd_objc::msg_send!(existing, zpd_objc::sel!("count"), () => usize);
                for index in 0..count {
                    let column = zpd_objc::msg_send!(existing, zpd_objc::sel!("objectAtIndex:"), ((index): usize) => Id);
                    zpd_objc::msg_send!(table, zpd_objc::sel!("removeTableColumn:"), ((column): Id) => ());
                }
                zpd_objc::msg_send!(existing, zpd_objc::sel!("release"), () => ());
                let list_column = TableColumn {
                    title: String::new(),
                    width: None,
                };
                let shown = if columns.is_empty() {
                    std::slice::from_ref(&list_column)
                } else {
                    columns
                };
                for (index, declared) in shown.iter().enumerate() {
                    let identifier = native::nsstring(&format!("column-{index}"));
                    let Some(column) = Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSTableColumn"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithIdentifier:"), ((identifier.as_ptr()): Id) => Id)) else {
                        continue;
                    };
                    let title = native::nsstring(&declared.title);
                    zpd_objc::msg_send!(column.as_ptr(), zpd_objc::sel!("setTitle:"), ((title.as_ptr()): Id) => ());
                    if let Some(width) = declared.width {
                        zpd_objc::msg_send!(column.as_ptr(), zpd_objc::sel!("setWidth:"), ((width): f64) => ());
                    }
                    zpd_objc::msg_send!(column.as_ptr(), zpd_objc::sel!("setResizingMask:"), ((native::NS_TABLE_COLUMN_AUTORESIZING_MASK | Self::USER_RESIZING_MASK): u64) => ());
                    zpd_objc::msg_send!(table, zpd_objc::sel!("addTableColumn:"), ((column.as_ptr()): Id) => ());
                }
                let header = if columns.is_empty() {
                    None
                } else {
                    Some(native::alloc_init(zpd_objc::class!("NSTableHeaderView")))
                };
                zpd_objc::msg_send!(table, zpd_objc::sel!("setHeaderView:"), ((header.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): Id) => ());
            }
        })?;
        Ok(())
    }

    const USER_RESIZING_MASK: u64 = 1 << 1;

    /// Shades every other row.
    pub fn set_striped(&self, striped: bool) -> Result<(), UiError> {
        self.table.with(|table| {
            // SAFETY: The Actor holds the live NSTableView.
            unsafe {
                zpd_objc::msg_send!(table, zpd_objc::sel!("setUsesAlternatingRowBackgroundColors:"), ((striped): bool) => ());
            }
        })?;
        Ok(())
    }

    pub fn set_style(&self, style: ListStyle) -> Result<(), UiError> {
        self.table.with(|table| {
            // SAFETY: The Actor holds the live NSTableView; the value is an NSTableViewStyle.
            unsafe {
                zpd_objc::msg_send!(table, zpd_objc::sel!("setStyle:"), ((style.native()): native::Integer) => ());
            }
        })?;
        Ok(())
    }

    /// Replaces the rows; `cells` holds one view per column of each [`ListEntry::Row`], row by row.
    ///
    /// Keeps the selected ID selected when it is still present, without reporting it.
    pub fn set_entries(&self, entries: &[ListEntry], cells: &[ViewRef<'_>]) -> Result<(), UiError> {
        let selected = self.selected_id()?;
        let columns = self.columns.get();
        let mut cells = cells.iter();
        let mut built = Vec::with_capacity(entries.len());
        for entry in entries {
            built.push(match entry {
                ListEntry::Header(title) => Entry {
                    id: None,
                    cells: vec![super::sidebar_table::cell(title, None, true)],
                    actors: Vec::new(),
                    height: ListState::HEADER_HEIGHT,
                },
                ListEntry::Row(id) => {
                    let tree = self.table.tree_handle().ok_or(UiError::Closed)?;
                    let mut wrappers = Vec::with_capacity(columns);
                    let mut actors = Vec::with_capacity(columns);
                    for _ in 0..columns {
                        let content = cells.next().ok_or(UiError::InvalidHierarchy)?;
                        let wrapper = view::new_layout_view(Rect::new(0.0, 0.0, 0.0, 0.0))?;
                        let actor = tree.insert_child(&self.table, wrapper.clone())?;
                        content.remove_from_superview()?;
                        ViewRef::from_actor(&actor).add_subview(content)?;
                        wrappers.push(wrapper);
                        actors.push(actor);
                    }
                    Entry {
                        id: Some(id.clone()),
                        cells: wrappers,
                        actors,
                        height: 44.0,
                    }
                }
            });
        }
        self.state.release_row_callbacks();
        let old = std::mem::replace(&mut *self.state.entries.borrow_mut(), built);
        // Old wrappers are empty now; their cells moved into the new ones.
        for actor in old.into_iter().flat_map(|entry| entry.actors) {
            actor.remove();
        }
        self.quietly(|table| {
            // SAFETY: The Actor holds the live NSTableView backed by this state.
            unsafe { zpd_objc::msg_send!(table, zpd_objc::sel!("reloadData"), () => ()) };
        })?;
        self.select(selected.as_deref())
    }

    /// Runs `callback` with the cell bounds whenever cell `index`, counted row by row, lays out.
    pub fn set_row_layout_handler(
        &self,
        index: usize,
        callback: impl FnMut(Rect, f64) + 'static,
    ) -> Result<(), UiError> {
        let cells = self.state.row_cells();
        let cell = cells.get(index).ok_or(UiError::InvalidHierarchy)?;
        // SAFETY: Row cells are live layout views created by `set_entries`.
        unsafe { view::set_layout_callback_on(*cell, callback) };
        Ok(())
    }

    /// Sets the height of each selectable row, in the order of `set_entries`, and lays rows out again.
    pub fn set_row_heights(&self, heights: &[f64]) -> Result<(), UiError> {
        let mut changed = Vec::new();
        {
            let mut entries = self.state.entries.borrow_mut();
            let rows = entries
                .iter_mut()
                .enumerate()
                .filter(|(_, entry)| entry.id.is_some());
            for ((row, entry), height) in rows.zip(heights) {
                if entry.height != *height {
                    entry.height = *height;
                    changed.push(row as u64);
                }
            }
        }
        let cells = self.state.row_cells();
        self.table.with(|table| {
            // SAFETY: The table and the row wrappers are live on the main thread.
            unsafe {
                if !changed.is_empty() {
                    let indexes = native::alloc_init(zpd_objc::class!("NSMutableIndexSet"));
                    for row in &changed {
                        zpd_objc::msg_send!(indexes.as_ptr(), zpd_objc::sel!("addIndex:"), ((*row): u64) => ());
                    }
                    zpd_objc::msg_send!(table, zpd_objc::sel!("noteHeightOfRowsWithIndexesChanged:"), ((indexes.as_ptr()): Id) => ());
                }
                for cell in cells {
                    zpd_objc::msg_send!(cell, zpd_objc::sel!("setNeedsLayout:"), ((true): bool) => ());
                }
            }
        })?;
        Ok(())
    }

    /// Returns the number of cells in each row.
    pub fn column_count(&self) -> usize {
        self.columns.get()
    }

    /// Returns the width available to the content of column `index`.
    pub fn column_width(&self, index: usize) -> Result<f64, UiError> {
        let width = self.table.with(|table| {
            // SAFETY: The live table returns the rectangle of one of its columns.
            unsafe {
                let rect = zpd_objc::msg_send!(table, zpd_objc::sel!("rectOfColumn:"), ((index as i64): i64) => native::Rect);
                rect.size.width
            }
        })?;
        Ok(width)
    }

    /// Scrolls the row with `id` into view.
    pub fn reveal(&self, id: &str) -> Result<(), UiError> {
        let Some(row) = self
            .state
            .entries
            .borrow()
            .iter()
            .position(|entry| entry.id.as_deref() == Some(id))
        else {
            return Ok(());
        };
        self.table.with(|table| {
            // SAFETY: The Actor holds the live NSTableView.
            unsafe {
                zpd_objc::msg_send!(table, zpd_objc::sel!("scrollRowToVisible:"), ((row as i64): i64) => ());
            }
        })?;
        Ok(())
    }

    /// Selects the row with `id`, or nothing, without reporting a user selection.
    pub fn select(&self, id: Option<&str>) -> Result<(), UiError> {
        let row = id.and_then(|id| {
            self.state
                .entries
                .borrow()
                .iter()
                .position(|entry| entry.id.as_deref() == Some(id))
        });
        self.quietly(|table| {
            // SAFETY: The Actor holds the live NSTableView backed by this state.
            unsafe {
                match row {
                    Some(row) => {
                        let indexes = zpd_objc::msg_send!(zpd_objc::class!("NSIndexSet"), zpd_objc::sel!("indexSetWithIndex:"), ((row as u64): u64) => Id);
                        zpd_objc::msg_send!(table, zpd_objc::sel!("selectRowIndexes:byExtendingSelection:"), ((indexes): Id, (false): bool) => ());
                    }
                    None => {
                        zpd_objc::msg_send!(table, zpd_objc::sel!("deselectAll:"), ((zpd_objc::NIL): Id) => ())
                    }
                }
            }
        })
    }

    pub fn selected_id(&self) -> Result<Option<String>, UiError> {
        let row = self.table.with(|table| {
            // SAFETY: The Actor holds the live NSTableView.
            unsafe { zpd_objc::msg_send!(table, zpd_objc::sel!("selectedRow"), () => i64) }
        })?;
        Ok(self.state.entry(row, |entry| entry.id.clone()))
    }

    /// Runs `operation` on the table without reporting selection changes.
    fn quietly(&self, operation: impl FnOnce(Id)) -> Result<(), UiError> {
        self.state.ready.set(false);
        let result = self.table.with(operation);
        self.state.ready.set(true);
        result.map_err(UiError::from)
    }
}

impl AsView for ListView {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}

impl Drop for ListView {
    fn drop(&mut self) {
        self.state.release_row_callbacks();
    }
}
