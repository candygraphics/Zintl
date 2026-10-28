use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;

use super::layout::{LayoutAttribute, LayoutRelation};
use super::{Sidebar, UiError};
use crate::actor::ActorRef;
use crate::native;
use zpd_objc::{Id, RcIvar, Strong};

struct Row {
    id: Option<String>,
    cell: Strong,
}

struct TableState {
    rows: RefCell<Vec<Row>>,
    ready: Cell<bool>,
    callback: RefCell<Box<dyn FnMut(&str)>>,
}

static TABLE_STATE: RcIvar<TableState> = RcIvar::new(c"_sidebarState");

impl TableState {
    fn with<R: Default>(object: Id, operation: impl FnOnce(&Self) -> R) -> R {
        zpd_objc::abort_on_unwind(|| {
            // SAFETY: Our registered delegate owns its state until teardown clears the ivar.
            let state = unsafe { TABLE_STATE.get(object) };
            state.as_deref().map(operation).unwrap_or_default()
        })
    }
}

unsafe extern "C" fn row_count(object: Id, _: zpd_objc::Sel, _: Id) -> i64 {
    TableState::with(object, |state| state.rows.borrow().len() as i64)
}

unsafe extern "C" fn row_view(object: Id, _: zpd_objc::Sel, _: Id, _: Id, row: i64) -> Id {
    TableState::with(object, |state| {
        state
            .rows
            .borrow()
            .get(row as usize)
            .map_or(zpd_objc::NIL, |row| row.cell.as_ptr())
    })
}

unsafe extern "C" fn is_group(object: Id, _: zpd_objc::Sel, _: Id, row: i64) -> bool {
    TableState::with(object, |state| {
        state
            .rows
            .borrow()
            .get(row as usize)
            .is_some_and(|row| row.id.is_none())
    })
}

unsafe extern "C" fn should_select(object: Id, _: zpd_objc::Sel, _: Id, row: i64) -> bool {
    TableState::with(object, |state| {
        state
            .rows
            .borrow()
            .get(row as usize)
            .is_some_and(|row| row.id.is_some())
    })
}

unsafe extern "C" fn selection_changed(object: Id, _: zpd_objc::Sel, notification: Id) {
    TableState::with(object, |state| {
        if !state.ready.get() {
            return;
        }
        // SAFETY: NSTableView sends this notification with itself as its object.
        let row = unsafe {
            let table =
                zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => zpd_objc::Id);
            zpd_objc::msg_send!(table, zpd_objc::sel!("selectedRow"), () => i64)
        };
        let id = state
            .rows
            .borrow()
            .get(row as usize)
            .and_then(|row| row.id.clone());
        if let Some(id) = id {
            (state.callback.borrow_mut())(&id);
        }
    });
}

fn release(object: Id) {
    // SAFETY: The caller passes a delegate created by `install`.
    drop(unsafe { TABLE_STATE.take(object) });
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
        zpd_objc::decl!(ZpdSidebarTableDelegate: [zpd_objc::class!("NSObject")] {
            fields { _sidebarState: ptr }
            methods {
                "numberOfRowsInTableView:": "q@:@" => row_count,
                "tableView:viewForTableColumn:row:": "@@:@@q" => row_view,
                "tableView:isGroupRow:": "B@:@q" => is_group,
                "tableView:shouldSelectRow:": "B@:@q" => should_select,
                "tableViewSelectionDidChange:": "v@:@" => selection_changed,
                "dealloc": "v@:" => dealloc,
            }
        }) as usize
    }) as zpd_objc::Class
}

pub(crate) fn constrain(
    first: Id,
    attribute: LayoutAttribute,
    second: Id,
    second_attribute: LayoutAttribute,
    constant: f64,
) {
    // SAFETY: All views share a cell ancestor; attributes and constants describe equalities.
    unsafe {
        let constraint = zpd_objc::msg_send!(zpd_objc::class!("NSLayoutConstraint"), zpd_objc::sel!("constraintWithItem:attribute:relatedBy:toItem:attribute:multiplier:constant:"), ((first): zpd_objc::Id, (attribute as i64): i64, (LayoutRelation::Equal as i64): i64, (second): zpd_objc::Id, (second_attribute as i64): i64, (1.0): f64, (constant): f64) => zpd_objc::Id);
        zpd_objc::msg_send!(constraint, zpd_objc::sel!("setActive:"), ((true): bool) => ());
    }
}

fn cell(title: &str, symbol: Option<&str>, group: bool) -> Strong {
    let cell = native::alloc_init(zpd_objc::class!("NSTableCellView"));
    let text = native::alloc_init(zpd_objc::class!("NSTextField"));
    // SAFETY: The retained cell owns its text/image subviews and active layout constraints.
    unsafe {
        zpd_objc::msg_send!(text.as_ptr(), zpd_objc::sel!("setStringValue:"), ((native::nsstring(title).as_ptr()): zpd_objc::Id) => ());
        zpd_objc::msg_send!(text.as_ptr(), zpd_objc::sel!("setEditable:"), ((false): bool) => ());
        zpd_objc::msg_send!(text.as_ptr(), zpd_objc::sel!("setSelectable:"), ((false): bool) => ());
        zpd_objc::msg_send!(text.as_ptr(), zpd_objc::sel!("setBordered:"), ((false): bool) => ());
        zpd_objc::msg_send!(text.as_ptr(), zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
        zpd_objc::msg_send!(text.as_ptr(), zpd_objc::sel!("setLineBreakMode:"), ((native::NS_LINE_BREAK_BY_TRUNCATING_TAIL): u64) => ());
        zpd_objc::msg_send!(text.as_ptr(), zpd_objc::sel!("setTranslatesAutoresizingMaskIntoConstraints:"), ((false): bool) => ());
        zpd_objc::msg_send!(cell.as_ptr(), zpd_objc::sel!("addSubview:"), ((text.as_ptr()): zpd_objc::Id) => ());
        zpd_objc::msg_send!(cell.as_ptr(), zpd_objc::sel!("setTextField:"), ((text.as_ptr()): zpd_objc::Id) => ());
        constrain(
            text.as_ptr(),
            LayoutAttribute::Trailing,
            cell.as_ptr(),
            LayoutAttribute::Trailing,
            -4.0,
        );
        constrain(
            text.as_ptr(),
            LayoutAttribute::CenterY,
            cell.as_ptr(),
            LayoutAttribute::CenterY,
            0.0,
        );
        if group {
            constrain(
                text.as_ptr(),
                LayoutAttribute::Leading,
                cell.as_ptr(),
                LayoutAttribute::Leading,
                4.0,
            );
        } else {
            let image = native::alloc_init(zpd_objc::class!("NSImageView"));
            zpd_objc::msg_send!(image.as_ptr(), zpd_objc::sel!("setTranslatesAutoresizingMaskIntoConstraints:"), ((false): bool) => ());
            zpd_objc::msg_send!(cell.as_ptr(), zpd_objc::sel!("addSubview:"), ((image.as_ptr()): zpd_objc::Id) => ());
            zpd_objc::msg_send!(cell.as_ptr(), zpd_objc::sel!("setImageView:"), ((image.as_ptr()): zpd_objc::Id) => ());
            let configuration = zpd_objc::msg_send!(zpd_objc::class!("NSImageSymbolConfiguration"), zpd_objc::sel!("configurationWithPointSize:weight:"), ((15.0): f64, (native::NSFontWeightRegular): f64) => zpd_objc::Id);
            zpd_objc::msg_send!(image.as_ptr(), zpd_objc::sel!("setSymbolConfiguration:"), ((configuration): zpd_objc::Id) => ());
            zpd_objc::msg_send!(image.as_ptr(), zpd_objc::sel!("setContentTintColor:"), ((zpd_objc::msg_send!(zpd_objc::class!("NSColor"), zpd_objc::sel!("labelColor"), () => zpd_objc::Id)): zpd_objc::Id) => ());
            if let Some(symbol) = symbol {
                let icon = zpd_objc::msg_send!(zpd_objc::class!("NSImage"), zpd_objc::sel!("imageWithSystemSymbolName:accessibilityDescription:"), ((native::nsstring(symbol).as_ptr()): zpd_objc::Id, (native::nsstring(title).as_ptr()): zpd_objc::Id) => zpd_objc::Id);
                zpd_objc::msg_send!(image.as_ptr(), zpd_objc::sel!("setImage:"), ((icon): zpd_objc::Id) => ());
            }
            constrain(
                image.as_ptr(),
                LayoutAttribute::Leading,
                cell.as_ptr(),
                LayoutAttribute::Leading,
                4.0,
            );
            constrain(
                image.as_ptr(),
                LayoutAttribute::CenterY,
                cell.as_ptr(),
                LayoutAttribute::CenterY,
                0.0,
            );
            constrain(
                image.as_ptr(),
                LayoutAttribute::Width,
                zpd_objc::NIL,
                LayoutAttribute::NotAnAttribute,
                18.0,
            );
            constrain(
                image.as_ptr(),
                LayoutAttribute::Height,
                zpd_objc::NIL,
                LayoutAttribute::NotAnAttribute,
                18.0,
            );
            constrain(
                text.as_ptr(),
                LayoutAttribute::Leading,
                image.as_ptr(),
                LayoutAttribute::Trailing,
                8.0,
            );
        }
    }
    cell
}

/// Builds table rows: section titles become group rows without IDs.
fn rows(sidebar: &Sidebar) -> Vec<Row> {
    let mut rows = Vec::new();
    for section in &sidebar.sections {
        if let Some(title) = &section.title {
            rows.push(Row {
                id: None,
                cell: cell(title, None, true),
            });
        }
        for item in &section.items {
            rows.push(Row {
                id: Some(item.id.clone()),
                cell: cell(&item.title, item.system_image.as_deref(), false),
            });
        }
    }
    rows
}

/// Selects the row for `selected_id` without reporting a user selection.
///
/// # Safety
/// `table` must be the live NSTableView backed by `state`.
unsafe fn select(table: Id, state: &TableState, selected_id: Option<&str>) {
    let selected = state
        .rows
        .borrow()
        .iter()
        .position(|row| row.id.is_some() && row.id.as_deref() == selected_id);
    // SAFETY: Guaranteed by the caller.
    unsafe {
        if let Some(row) = selected {
            let indexes = zpd_objc::msg_send!(zpd_objc::class!("NSIndexSet"), zpd_objc::sel!("indexSetWithIndex:"), ((row as u64): u64) => zpd_objc::Id);
            zpd_objc::msg_send!(table, zpd_objc::sel!("selectRowIndexes:byExtendingSelection:"), ((indexes): zpd_objc::Id, (false): bool) => ());
        } else {
            zpd_objc::msg_send!(table, zpd_objc::sel!("deselectAll:"), ((zpd_objc::NIL): zpd_objc::Id) => ());
        }
    }
}

/// The installed sidebar table, which can be updated without reinstalling.
pub(crate) struct SidebarTable {
    state: Rc<TableState>,
    table: ActorRef,
}
impl SidebarTable {
    pub(crate) fn update(&self, sidebar: &Sidebar) -> Result<(), UiError> {
        self.state.ready.set(false);
        *self.state.rows.borrow_mut() = rows(sidebar);
        let result = self.table.with(|table| {
            // SAFETY: The Actor holds the live table backed by this state.
            unsafe {
                zpd_objc::msg_send!(table, zpd_objc::sel!("reloadData"), () => ());
                select(table, &self.state, sidebar.selected_id.as_deref());
            }
        });
        self.state.ready.set(true);
        result.map_err(UiError::from)
    }
}

pub(crate) fn install(
    controller: &ActorRef,
    sidebar: &Sidebar,
    callback: impl FnMut(&str) + 'static,
) -> Result<SidebarTable, UiError> {
    let tree = controller.tree_handle().ok_or(UiError::Closed)?;
    let scroll = native::alloc_init(zpd_objc::class!("NSScrollView"));
    let scroll_actor = tree
        .insert_child(controller, scroll.clone())
        .map_err(UiError::from)?;
    let table = native::alloc_init(zpd_objc::class!("NSTableView"));
    let table_actor = tree
        .insert_child(&scroll_actor, table.clone())
        .map_err(UiError::from)?;
    let state = Rc::new(TableState {
        rows: RefCell::new(rows(sidebar)),
        ready: Cell::new(false),
        callback: RefCell::new(Box::new(callback)),
    });
    let delegate = native::alloc_init(delegate_class());
    // SAFETY: The registered delegate class declares the `_sidebarState` ivar.
    unsafe { TABLE_STATE.set(delegate.as_ptr(), state.clone()) };
    let delegate_actor = tree
        .insert_child(&table_actor, delegate.clone())
        .map_err(UiError::from)?;
    tree.add_teardown(&delegate_actor, release)
        .map_err(UiError::from)?;
    tree.add_teardown(&table_actor, |table| {
        // SAFETY: Disconnect non-owning AppKit delegates before their Rust state is released.
        unsafe {
            zpd_objc::msg_send!(table, zpd_objc::sel!("setDelegate:"), ((zpd_objc::NIL): zpd_objc::Id) => ());
            zpd_objc::msg_send!(table, zpd_objc::sel!("setDataSource:"), ((zpd_objc::NIL): zpd_objc::Id) => ());
        }
    })
    .map_err(UiError::from)?;
    // SAFETY: The controller, scroll view, table, column and delegate are live on the main thread.
    unsafe {
        zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
        zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setHasVerticalScroller:"), ((true): bool) => ());
        zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setAutohidesScrollers:"), ((true): bool) => ());
        let column = Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSTableColumn"), zpd_objc::sel!("alloc"), () => zpd_objc::Id), zpd_objc::sel!("initWithIdentifier:"), ((native::nsstring("items").as_ptr()): zpd_objc::Id) => zpd_objc::Id))
        .ok_or(UiError::NativeCreationFailed)?;
        zpd_objc::msg_send!(column.as_ptr(), zpd_objc::sel!("setResizingMask:"), ((native::NS_TABLE_COLUMN_AUTORESIZING_MASK): u64) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("addTableColumn:"), ((column.as_ptr()): zpd_objc::Id) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setHeaderView:"), ((zpd_objc::NIL): zpd_objc::Id) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setStyle:"), ((native::NS_TABLE_VIEW_STYLE_SOURCE_LIST): i64) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setRowSizeStyle:"), ((native::NS_TABLE_VIEW_ROW_SIZE_STYLE_CUSTOM): i64) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setBackgroundColor:"), ((zpd_objc::msg_send!(zpd_objc::class!("NSColor"), zpd_objc::sel!("clearColor"), () => zpd_objc::Id)): zpd_objc::Id) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setRowHeight:"), ((32.0): f64) => ());

        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setAllowsMultipleSelection:"), ((false): bool) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setDataSource:"), ((delegate.as_ptr()): zpd_objc::Id) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("setDelegate:"), ((delegate.as_ptr()): zpd_objc::Id) => ());
        zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDocumentView:"), ((table.as_ptr()): zpd_objc::Id) => ());
        zpd_objc::msg_send!(table.as_ptr(), zpd_objc::sel!("reloadData"), () => ());
        select(table.as_ptr(), &state, sidebar.selected_id.as_deref());
    }
    controller
        .with(|controller| {
            // SAFETY: The controller retains the installed scroll view.
            unsafe {
                zpd_objc::msg_send!(controller, zpd_objc::sel!("setView:"), ((scroll.as_ptr()): zpd_objc::Id) => ());
            }
        })
        .map_err(UiError::from)?;
    state.ready.set(true);
    Ok(SidebarTable {
        state,
        table: table_actor,
    })
}
