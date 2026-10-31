use crate::actor::{ActorRef, ActorRollback, WindowEventKind};
use crate::native;
use zpd_objc::{Id, Strong};

use super::UiError;
use super::callback;
use super::layout::LayoutAttribute;
use super::sidebar_table::{SidebarTable, constrain};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sidebar {
    pub sections: Vec<SidebarSection>,
    pub selected_id: Option<String>,
    /// Shows a search field above the items.
    pub search: Option<SidebarSearch>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SidebarSearch {
    pub value: String,
    pub placeholder: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SidebarSection {
    pub title: Option<String>,
    pub items: Vec<SidebarItem>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SidebarItem {
    pub id: String,
    pub title: String,
    pub system_image: Option<String>,
    /// Trailing text such as a count.
    pub badge: Option<String>,
    /// The item's context menu; selections are reported from the window.
    pub menu: Vec<super::CommandItem>,
}

pub(crate) struct SidebarNative {
    content_controller: ActorRef,
    root: ActorRef,
    item: ActorRef,
    table: SidebarTable,
    search: Option<ActorRef>,
}
impl SidebarNative {
    /// Returns whether `sidebar` can be applied without replacing native views.
    pub(crate) fn can_update(&self, sidebar: &Sidebar) -> bool {
        self.search.is_some() == sidebar.search.is_some()
    }

    pub(crate) fn update(&self, sidebar: &Sidebar) -> Result<(), UiError> {
        self.table.update(sidebar)?;
        if let (Some(field), Some(search)) = (&self.search, &sidebar.search) {
            field.with(|field| {
                // SAFETY: The Actor holds the live NSSearchField; equal values are left alone
                // so typing keeps its insertion point.
                unsafe { configure_search(field, search) }
            })?;
        }
        Ok(())
    }

    pub(crate) fn is_collapsed(&self) -> bool {
        self.item
            .with(|item| {
                // SAFETY: item is a live NSSplitViewItem on the main thread.
                unsafe { zpd_objc::msg_send!(item, zpd_objc::sel!("isCollapsed"), () => bool) }
            })
            .unwrap_or(false)
    }

    pub(crate) fn clear(self, window: &ActorRef) -> Result<(), UiError> {
        window
            .with(|window| {
                // SAFETY: AppKit objects and selectors have matching types and run on the main thread.
                self.content_controller.with(|controller| unsafe {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("setContentViewController:"), ((controller): zpd_objc::Id) => ());
                })
            })
            .map_err(UiError::from)?
            .map_err(UiError::from)?;
        self.root.remove();
        Ok(())
    }
}

impl Drop for SidebarNative {
    fn drop(&mut self) {
        self.root.remove();
    }
}

/// Applies search text and placeholder to a live `NSSearchField`.
///
/// # Safety
/// `field` must be a live `NSSearchField`.
unsafe fn configure_search(field: Id, search: &SidebarSearch) {
    // SAFETY: Guaranteed by the caller.
    unsafe {
        let current = native::rust_string(
            zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => Id),
        );
        if current != search.value {
            let value = native::nsstring(&search.value);
            zpd_objc::msg_send!(field, zpd_objc::sel!("setStringValue:"), ((value.as_ptr()): Id) => ());
        }
        let placeholder = search.placeholder.as_deref().map(native::nsstring);
        zpd_objc::msg_send!(field, zpd_objc::sel!("setPlaceholderString:"), ((placeholder.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): Id) => ());
    }
}

/// Wraps the controller's table in a view with a search field above it.
fn install_search(
    window: &ActorRef,
    controller: &ActorRef,
    search: &SidebarSearch,
) -> Result<ActorRef, UiError> {
    let tree = controller.tree_handle().ok_or(UiError::Closed)?;
    let container = native::alloc_init(zpd_objc::class!("NSView"));
    let field = native::alloc_init(zpd_objc::class!("NSSearchField"));
    controller.with(|controller| {
        // SAFETY: The controller's view is the sidebar scroll view installed by the table.
        unsafe {
            let scroll = zpd_objc::msg_send!(controller, zpd_objc::sel!("view"), () => Id);
            configure_search(field.as_ptr(), search);
            for view in [field.as_ptr(), scroll] {
                zpd_objc::msg_send!(view, zpd_objc::sel!("setTranslatesAutoresizingMaskIntoConstraints:"), ((false): bool) => ());
                zpd_objc::msg_send!(container.as_ptr(), zpd_objc::sel!("addSubview:"), ((view): Id) => ());
            }
            zpd_objc::msg_send!(controller, zpd_objc::sel!("setView:"), ((container.as_ptr()): Id) => ());
            // The safe-area guide keeps the field below the title bar in full-height sidebars.
            let guide = zpd_objc::msg_send!(container.as_ptr(), zpd_objc::sel!("safeAreaLayoutGuide"), () => Id);
            let top = |view: Id| zpd_objc::msg_send!(view, zpd_objc::sel!("topAnchor"), () => Id);
            let anchor_constraint = zpd_objc::msg_send!(top(field.as_ptr()), zpd_objc::sel!("constraintEqualToAnchor:constant:"), ((top(guide)): Id, (4.0): f64) => Id);
            zpd_objc::msg_send!(anchor_constraint, zpd_objc::sel!("setActive:"), ((true): bool) => ());
            constrain(field.as_ptr(), LayoutAttribute::Leading, container.as_ptr(), LayoutAttribute::Leading, 10.0);
            constrain(field.as_ptr(), LayoutAttribute::Trailing, container.as_ptr(), LayoutAttribute::Trailing, -10.0);
            constrain(scroll, LayoutAttribute::Top, field.as_ptr(), LayoutAttribute::Bottom, 6.0);
            constrain(scroll, LayoutAttribute::Leading, container.as_ptr(), LayoutAttribute::Leading, 0.0);
            constrain(scroll, LayoutAttribute::Trailing, container.as_ptr(), LayoutAttribute::Trailing, 0.0);
            constrain(scroll, LayoutAttribute::Bottom, container.as_ptr(), LayoutAttribute::Bottom, 0.0);
        }
    })?;
    tree.insert_child(controller, container)?;
    let field = tree.insert_child(controller, field)?;
    // Search edits are reported by the Window like sidebar selection changes.
    let emit = callback::emitter(window);
    callback::install_delegate(&field, move |notification| {
        // SAFETY: Text notifications carry the live NSSearchField as their object.
        let value = unsafe {
            let field = zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id);
            native::rust_string(zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => Id))
        };
        emit(WindowEventKind::SidebarSearchChanged { value });
    })?;
    let emit = callback::emitter(window);
    callback::install_action(&field, move |field| {
        // SAFETY: The action sender is the live NSSearchField.
        let value = unsafe {
            native::rust_string(zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => Id))
        };
        emit(WindowEventKind::SidebarSearchChanged { value });
    })?;
    Ok(field)
}

pub(crate) fn install<F>(
    window: &ActorRef,
    content_controller: &ActorRef,
    sidebar: &Sidebar,
    collapsed: bool,
    callback_fn: F,
) -> Result<SidebarNative, UiError>
where
    F: FnMut(&str) + 'static,
{
    let tree = window.tree_handle().ok_or(UiError::Closed)?;
    let split = native::alloc_init(zpd_objc::class!("NSSplitViewController"));
    let root = tree.insert_child(window, split).map_err(UiError::from)?;
    let mut rollback = ActorRollback::new(root.clone());
    let sidebar_controller = native::alloc_init(zpd_objc::class!("NSViewController"));
    let sidebar_controller_actor = tree
        .insert_child(&root, sidebar_controller.clone())
        .map_err(UiError::from)?;
    let table =
        super::sidebar_table::install(window, &sidebar_controller_actor, sidebar, callback_fn)?;
    let search = sidebar
        .search
        .as_ref()
        .map(|search| install_search(window, &sidebar_controller_actor, search))
        .transpose()?;
    // SAFETY: AppKit objects and selectors have matching types and run on the main thread.
    let sidebar_item = unsafe {
        Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("NSSplitViewItem"), zpd_objc::sel!("sidebarWithViewController:"), ((sidebar_controller.as_ptr()): zpd_objc::Id) => zpd_objc::Id))
    }
    .ok_or(UiError::NativeCreationFailed)?;
    // SAFETY: AppKit objects and selectors have matching types and run on the main thread.
    let content_item = unsafe {
        Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("NSSplitViewItem"), zpd_objc::sel!("splitViewItemWithViewController:"), ((content_controller
                .with(|id| id)
                .map_err(UiError::from)?): zpd_objc::Id) => zpd_objc::Id))
    }
    .ok_or(UiError::NativeCreationFailed)?;
    // SAFETY: AppKit objects and selectors have matching types and run on the main thread.
    unsafe {
        zpd_objc::msg_send!(sidebar_item.as_ptr(), zpd_objc::sel!("setMinimumThickness:"), ((180.0): f64) => ());
        zpd_objc::msg_send!(sidebar_item.as_ptr(), zpd_objc::sel!("setMaximumThickness:"), ((320.0): f64) => ());
        zpd_objc::msg_send!(root.with(|id| id).map_err(UiError::from)?, zpd_objc::sel!("addSplitViewItem:"), ((sidebar_item.as_ptr()): zpd_objc::Id) => ());
        zpd_objc::msg_send!(root.with(|id| id).map_err(UiError::from)?, zpd_objc::sel!("addSplitViewItem:"), ((content_item.as_ptr()): zpd_objc::Id) => ());
    }
    // SAFETY: sidebar_item is a retained NSSplitViewItem with a live controller.
    unsafe {
        zpd_objc::msg_send!(sidebar_item.as_ptr(), zpd_objc::sel!("setAllowsFullHeightLayout:"), ((true): bool) => ());
        zpd_objc::msg_send!(content_item.as_ptr(), zpd_objc::sel!("setMinimumThickness:"), ((360.0): f64) => ());
        zpd_objc::msg_send!(sidebar_item.as_ptr(), zpd_objc::sel!("setCanCollapse:"), ((true): bool) => ());
        zpd_objc::msg_send!(sidebar_item.as_ptr(), zpd_objc::sel!("setCollapsed:"), ((collapsed): bool) => ());
    }
    window
        .with(|window| {
            // SAFETY: AppKit objects and selectors have matching types and run on the main thread.
            root.with(|split| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("setContentViewController:"), ((split): zpd_objc::Id) => ())
            })
        })
        .map_err(UiError::from)?
        .map_err(UiError::from)?;
    let item = tree
        .insert_child(&root, sidebar_item)
        .map_err(UiError::from)?;
    tree.insert_child(&root, content_item)
        .map_err(UiError::from)?;
    rollback.disarm();
    Ok(SidebarNative {
        content_controller: content_controller.clone(),
        root,
        item,
        table,
        search,
    })
}
