//! A window toolbar with buttons, a search field, and flexible space.

use std::rc::Rc;
use std::sync::OnceLock;

use crate::actor::{ActorRef, WindowEventKind};
use crate::native;
use zpd_objc::{Id, RcIvar, Strong};

use super::{UiError, callback};

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    static NSToolbarToggleSidebarItemIdentifier: Id;
    static NSToolbarSidebarTrackingSeparatorItemIdentifier: Id;
    static NSToolbarFlexibleSpaceItemIdentifier: Id;
}

/// The items of a window toolbar, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Toolbar {
    pub items: Vec<ToolbarItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolbarItem {
    /// A button showing an SF Symbol; clicks emit [`WindowEventKind::ToolbarItemClicked`].
    Button {
        id: String,
        label: String,
        system_image: String,
        enabled: bool,
    },
    /// A search field; edits emit [`WindowEventKind::ToolbarSearchChanged`].
    Search {
        value: String,
        placeholder: Option<String>,
    },
    FlexibleSpace,
}

impl ToolbarItem {
    /// Returns whether `other` can update this item's native view in place.
    fn same_kind(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Button { id, .. }, Self::Button { id: other, .. }) => id == other,
            (Self::Search { .. }, Self::Search { .. })
            | (Self::FlexibleSpace, Self::FlexibleSpace) => true,
            _ => false,
        }
    }
}

struct ToolbarState {
    /// Identifiers in display order, including system items.
    identifiers: Strong,
    /// Items this toolbar created, by identifier.
    items: Vec<(String, Strong)>,
}

static TOOLBAR_STATE: RcIvar<ToolbarState> = RcIvar::new(c"_zpdToolbarState");

impl ToolbarState {
    fn with<R: Default>(object: Id, operation: impl FnOnce(&Self) -> R) -> R {
        zpd_objc::abort_on_unwind(|| {
            // SAFETY: Our delegate owns its state until teardown clears the ivar.
            let state = unsafe { TOOLBAR_STATE.get(object) };
            state.as_deref().map(operation).unwrap_or_default()
        })
    }
}

unsafe extern "C" fn item_identifiers(object: Id, _: zpd_objc::Sel, _: Id) -> Id {
    ToolbarState::with(object, |state| Some(state.identifiers.as_ptr())).unwrap_or(zpd_objc::NIL)
}

unsafe extern "C" fn item_for_identifier(
    object: Id,
    _: zpd_objc::Sel,
    _: Id,
    identifier: Id,
    _: bool,
) -> Id {
    ToolbarState::with(object, |state| {
        // SAFETY: AppKit passes a live NSString identifier.
        let identifier = unsafe { native::rust_string(identifier) };
        state
            .items
            .iter()
            .find(|(id, _)| *id == identifier)
            .map(|(_, item)| item.as_ptr())
    })
    .unwrap_or(zpd_objc::NIL)
}

fn release(object: Id) {
    // SAFETY: The caller passes a delegate created by `WindowToolbar::install`.
    drop(unsafe { TOOLBAR_STATE.take(object) });
}

unsafe extern "C" fn dealloc(object: Id, _: zpd_objc::Sel) {
    zpd_objc::abort_on_unwind(|| release(object));
    // SAFETY: Our NSObject subclass has released its Rust state.
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSObject"), zpd_objc::sel!("dealloc"), () => ());
    }
}

/// The toolbar installed on a window and the Actors of its controls.
pub(crate) struct WindowToolbar {
    declared: Toolbar,
    sidebar: bool,
    root: ActorRef,
    /// The native item for each declared item, in order.
    natives: Vec<Option<ActorRef>>,
}

impl WindowToolbar {
    fn delegate_class() -> zpd_objc::Class {
        static CLASS: OnceLock<usize> = OnceLock::new();
        *CLASS.get_or_init(|| {
            zpd_objc::decl!(ZpdToolbarDelegate: [zpd_objc::class!("NSObject")] {
                fields { _zpdToolbarState: ptr }
                methods {
                    "toolbarDefaultItemIdentifiers:": "@@:@" => item_identifiers,
                    "toolbarAllowedItemIdentifiers:": "@@:@" => item_identifiers,
                    "toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:": "@@:@@B" => item_for_identifier,
                    "dealloc": "v@:" => dealloc,
                }
            }) as usize
        }) as zpd_objc::Class
    }

    /// Returns whether `toolbar` can be applied by updating items in place.
    pub(crate) fn can_update(&self, toolbar: &Toolbar, sidebar: bool) -> bool {
        self.sidebar == sidebar
            && self.declared.items.len() == toolbar.items.len()
            && self
                .declared
                .items
                .iter()
                .zip(&toolbar.items)
                .all(|(old, new)| old.same_kind(new))
    }

    /// Updates enabled states and search text without rebuilding the toolbar.
    pub(crate) fn update(&mut self, toolbar: &Toolbar) -> Result<(), UiError> {
        for (item, native) in toolbar.items.iter().zip(&self.natives) {
            let Some(native) = native else {
                continue;
            };
            native.with(|native| {
                // SAFETY: Each Actor holds the live item or search field it was created for.
                unsafe {
                    match item {
                        ToolbarItem::Button { enabled, .. } => {
                            zpd_objc::msg_send!(native, zpd_objc::sel!("setEnabled:"), ((*enabled): bool) => ());
                        }
                        ToolbarItem::Search { value, placeholder } => {
                            Self::configure_search(native, value, placeholder.as_deref());
                        }
                        ToolbarItem::FlexibleSpace => {}
                    }
                }
            })?;
        }
        self.declared = toolbar.clone();
        Ok(())
    }

    /// Applies search text and placeholder; equal text is left alone to keep the caret.
    ///
    /// # Safety
    /// `field` must be a live `NSSearchField`.
    unsafe fn configure_search(field: Id, value: &str, placeholder: Option<&str>) {
        // SAFETY: Guaranteed by the caller.
        unsafe {
            let current = native::rust_string(
                zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => Id),
            );
            if current != value {
                zpd_objc::msg_send!(field, zpd_objc::sel!("setStringValue:"), ((native::nsstring(value).as_ptr()): Id) => ());
            }
            let placeholder = placeholder.map(native::nsstring);
            zpd_objc::msg_send!(field, zpd_objc::sel!("setPlaceholderString:"), ((placeholder.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): Id) => ());
        }
    }

    /// Builds the toolbar and installs it on `window`, replacing any toolbar there.
    ///
    /// With a sidebar, the sidebar toggle and tracking separator come first,
    /// so declared items sit over the content.
    pub(crate) fn install(
        window: &ActorRef,
        toolbar: &Toolbar,
        sidebar: bool,
    ) -> Result<Self, UiError> {
        let tree = window.tree_handle().ok_or(UiError::Closed)?;
        let identifier = native::nsstring("zintl.window.toolbar");
        // SAFETY: NSToolbar's designated initializer takes an identifier string.
        let native_toolbar = unsafe {
            Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSToolbar"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithIdentifier:"), ((identifier.as_ptr()): Id) => Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        let root = tree.insert_child(window, native_toolbar.clone())?;
        let emit = Rc::new(callback::emitter(window));
        let identifiers = native::alloc_init(zpd_objc::class!("NSMutableArray"));
        let add = |identifier: Id| {
            // SAFETY: The array retains the identifier string.
            unsafe {
                zpd_objc::msg_send!(identifiers.as_ptr(), zpd_objc::sel!("addObject:"), ((identifier): Id) => ());
            }
        };
        if sidebar {
            // SAFETY: AppKit exports these identifiers as immutable strings.
            unsafe {
                add(NSToolbarToggleSidebarItemIdentifier);
                add(NSToolbarSidebarTrackingSeparatorItemIdentifier);
            }
        }
        let mut items = Vec::new();
        let mut natives = Vec::new();
        for (index, item) in toolbar.items.iter().enumerate() {
            let name = format!("zintl.toolbar.{index}");
            let native_name = native::nsstring(&name);
            let created = match item {
                ToolbarItem::FlexibleSpace => {
                    // SAFETY: AppKit exports the identifier as an immutable string.
                    unsafe { add(NSToolbarFlexibleSpaceItemIdentifier) };
                    natives.push(None);
                    continue;
                }
                ToolbarItem::Button {
                    id,
                    label,
                    system_image,
                    enabled,
                } => {
                    // SAFETY: NSToolbarItem's initializer copies the identifier; the image is autoreleased.
                    let item = unsafe {
                        let item = Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSToolbarItem"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithItemIdentifier:"), ((native_name.as_ptr()): Id) => Id))
                            .ok_or(UiError::NativeCreationFailed)?;
                        let label = native::nsstring(label);
                        zpd_objc::msg_send!(item.as_ptr(), zpd_objc::sel!("setLabel:"), ((label.as_ptr()): Id) => ());
                        zpd_objc::msg_send!(item.as_ptr(), zpd_objc::sel!("setToolTip:"), ((label.as_ptr()): Id) => ());
                        let image = native::symbol_image(
                            system_image,
                            &native::rust_string(label.as_ptr()),
                        );
                        zpd_objc::msg_send!(item.as_ptr(), zpd_objc::sel!("setImage:"), ((image): Id) => ());
                        zpd_objc::msg_send!(item.as_ptr(), zpd_objc::sel!("setBordered:"), ((true): bool) => ());
                        zpd_objc::msg_send!(item.as_ptr(), zpd_objc::sel!("setAutovalidates:"), ((false): bool) => ());
                        zpd_objc::msg_send!(item.as_ptr(), zpd_objc::sel!("setEnabled:"), ((*enabled): bool) => ());
                        item
                    };
                    let actor = tree.insert_child(&root, item.clone())?;
                    let emit = emit.clone();
                    let id = id.clone();
                    callback::install_action(&actor, move |_| {
                        emit(WindowEventKind::ToolbarItemClicked { id: id.clone() })
                    })?;
                    natives.push(Some(actor));
                    item
                }
                ToolbarItem::Search { value, placeholder } => {
                    // SAFETY: NSSearchToolbarItem owns its search field.
                    let (item, field) = unsafe {
                        let item = Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSSearchToolbarItem"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithItemIdentifier:"), ((native_name.as_ptr()): Id) => Id))
                            .ok_or(UiError::NativeCreationFailed)?;
                        let field = zpd_objc::msg_send!(item.as_ptr(), zpd_objc::sel!("searchField"), () => Id);
                        Self::configure_search(field, value, placeholder.as_deref());
                        (
                            item,
                            Strong::retain(field).ok_or(UiError::NativeCreationFailed)?,
                        )
                    };
                    tree.insert_child(&root, item.clone())?;
                    let field = tree.insert_child(&root, field)?;
                    let emit = emit.clone();
                    callback::install_delegate(&field, move |notification| {
                        // SAFETY: Text notifications carry the live search field as their object.
                        let value = unsafe {
                            let field = zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id);
                            native::rust_string(
                                zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => Id),
                            )
                        };
                        emit(WindowEventKind::ToolbarSearchChanged { value });
                    })?;
                    natives.push(Some(field));
                    item
                }
            };
            add(native_name.as_ptr());
            items.push((name, created));
        }
        let state = Rc::new(ToolbarState {
            identifiers: identifiers.clone(),
            items,
        });
        let delegate = native::alloc_init(Self::delegate_class());
        // SAFETY: The registered delegate class declares the `_zpdToolbarState` ivar.
        unsafe { TOOLBAR_STATE.set(delegate.as_ptr(), state) };
        let delegate_actor = tree.insert_child(&root, delegate.clone())?;
        tree.add_teardown(&delegate_actor, release)?;
        tree.add_teardown(&root, |toolbar| {
            // SAFETY: Detach the unretained delegate before its state is released.
            unsafe {
                zpd_objc::msg_send!(toolbar, zpd_objc::sel!("setDelegate:"), ((zpd_objc::NIL): Id) => ());
            }
        })?;
        window.with(|window| {
            // SAFETY: The window retains its toolbar; the delegate stays alive as a child Actor.
            unsafe {
                zpd_objc::msg_send!(native_toolbar.as_ptr(), zpd_objc::sel!("setDelegate:"), ((delegate.as_ptr()): Id) => ());
                zpd_objc::msg_send!(native_toolbar.as_ptr(), zpd_objc::sel!("setDisplayMode:"), ((native::NS_TOOLBAR_DISPLAY_MODE_ICON_ONLY): u64) => ());
                zpd_objc::msg_send!(window, zpd_objc::sel!("setToolbar:"), ((native_toolbar.as_ptr()): Id) => ());
            }
        })?;
        Ok(Self {
            declared: toolbar.clone(),
            sidebar,
            root,
            natives,
        })
    }

    /// Removes the toolbar from `window` if it is still installed.
    pub(crate) fn clear(self, window: &ActorRef) -> Result<(), UiError> {
        let toolbar = self.root.with(|toolbar| toolbar)?;
        window.with(|window| {
            // SAFETY: The window and toolbar are live while this Actor exists.
            unsafe {
                if zpd_objc::msg_send!(window, zpd_objc::sel!("toolbar"), () => Id) == toolbar {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("setToolbar:"), ((zpd_objc::NIL): Id) => ());
                }
            }
        })?;
        self.root.remove();
        Ok(())
    }
}
