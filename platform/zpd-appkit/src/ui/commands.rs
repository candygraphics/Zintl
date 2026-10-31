use std::cell::RefCell;
use std::rc::Rc;

use crate::actor::{ActorRef, ActorTree};
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, Strong};

use super::UiError;
use super::callback;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandSet {
    pub app_menu: Option<WindowAppMenu>,
    pub menus: Vec<CommandMenu>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowAppMenu {
    pub items: Vec<CommandItem>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandMenu {
    pub title: String,
    pub items: Vec<CommandItem>,
    /// Registers the menu with `NSApp` so AppKit can manage its contents.
    pub role: Option<MenuRole>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CommandItem {
    pub id: Option<String>,
    pub title: String,
    pub role: Option<CommandRole>,
    pub key: Option<String>,
    pub modifiers: Vec<CommandModifier>,
    pub enabled: bool,
    pub checked: bool,
    pub submenu: Option<CommandMenu>,
}
impl CommandItem {
    /// Creates an enabled item with no action.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: None,
            title: title.into(),
            role: None,
            key: None,
            modifiers: Vec::new(),
            enabled: true,
            checked: false,
            submenu: None,
        }
    }
    pub fn separator() -> Self {
        Self {
            role: Some(CommandRole::Separator),
            ..Self::new("")
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandModifier {
    Cmd,
    Ctrl,
    Alt,
    Shift,
}
/// Standard actions delivered through the AppKit responder chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandRole {
    About,
    Quit,
    Hide,
    HideOthers,
    ShowAll,
    Close,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    /// Text finder actions, which `NSTextView` handles with its find bar.
    Find,
    FindAndReplace,
    FindNext,
    FindPrevious,
    UseSelectionForFind,
    /// Font traits, which the shared `NSFontManager` applies to the selection.
    Bold,
    Italic,
    Underline,
    ToggleSidebar,
    ToggleFullScreen,
    Minimize,
    Zoom,
    BringAllToFront,
    Help,
    Separator,
}
/// Menus that `NSApp` fills or decorates itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuRole {
    Services,
    Windows,
    Help,
}

impl CommandRole {
    fn selector(self) -> Option<zpd_objc::Sel> {
        Some(match self {
            Self::About => zpd_objc::sel!("orderFrontStandardAboutPanel:"),
            Self::Quit => zpd_objc::sel!("terminate:"),
            Self::Hide => zpd_objc::sel!("hide:"),
            Self::HideOthers => zpd_objc::sel!("hideOtherApplications:"),
            Self::ShowAll => zpd_objc::sel!("unhideAllApplications:"),
            Self::Close => zpd_objc::sel!("performClose:"),
            Self::Undo => zpd_objc::sel!("undo:"),
            Self::Redo => zpd_objc::sel!("redo:"),
            Self::Cut => zpd_objc::sel!("cut:"),
            Self::Copy => zpd_objc::sel!("copy:"),
            Self::Paste => zpd_objc::sel!("paste:"),
            Self::Delete => zpd_objc::sel!("delete:"),
            Self::SelectAll => zpd_objc::sel!("selectAll:"),
            Self::Find
            | Self::FindAndReplace
            | Self::FindNext
            | Self::FindPrevious
            | Self::UseSelectionForFind => zpd_objc::sel!("performTextFinderAction:"),
            Self::Bold | Self::Italic => zpd_objc::sel!("addFontTrait:"),
            Self::Underline => zpd_objc::sel!("underline:"),
            Self::ToggleSidebar => zpd_objc::sel!("toggleSidebar:"),
            Self::ToggleFullScreen => zpd_objc::sel!("toggleFullScreen:"),
            Self::Minimize => zpd_objc::sel!("performMiniaturize:"),
            Self::Zoom => zpd_objc::sel!("performZoom:"),
            Self::BringAllToFront => zpd_objc::sel!("arrangeInFront:"),
            Self::Help => zpd_objc::sel!("showHelp:"),
            Self::Separator => return None,
        })
    }

    /// Returns the menu item tag the action reads, such as an `NSTextFinderAction`.
    fn tag(self) -> native::Integer {
        match self {
            Self::Find => 1,
            Self::FindNext => 2,
            Self::FindPrevious => 3,
            Self::UseSelectionForFind => 7,
            Self::FindAndReplace => 12,
            // NSFontTraitMask values.
            Self::Bold => 2,
            Self::Italic => 1,
            _ => 0,
        }
    }

    /// Returns the fixed target, or nil to use the responder chain.
    fn target(self) -> Id {
        match self {
            // SAFETY: The shared font manager is a process-wide singleton.
            Self::Bold | Self::Italic => unsafe {
                zpd_objc::msg_send!(zpd_objc::class!("NSFontManager"), zpd_objc::sel!("sharedFontManager"), () => Id)
            },
            _ => zpd_objc::NIL,
        }
    }
}

pub(crate) type MenuCallback = Rc<RefCell<dyn FnMut(&str)>>;

pub(crate) fn install<F>(
    application: &Application,
    commands: &CommandSet,
    callback_fn: F,
) -> Result<(), UiError>
where
    F: FnMut(&str) + 'static,
{
    let callback_fn: MenuCallback = Rc::new(RefCell::new(callback_fn));
    let tree = application.tree();
    let app = application.actor_ref();
    app.with(|app| unsafe {
        zpd_objc::msg_send!(app, zpd_objc::sel!("setMainMenu:"), ((zpd_objc::NIL): zpd_objc::Id) => ())
    })
    .map_err(UiError::from)?;
    let main_menu = native::alloc_init(zpd_objc::class!("NSMenu"));
    let main = tree
        .replace_owned(&app, "commands", main_menu)
        .map_err(UiError::from)?;
    let build_result = (|| {
        if let Some(menu) = &commands.app_menu {
            let menu = CommandMenu {
                title: String::new(),
                items: menu.items.clone(),
                role: None,
            };
            add_submenu(tree, &main, &menu, &callback_fn)?;
        }
        for menu in &commands.menus {
            add_submenu(tree, &main, menu, &callback_fn)?;
        }
        Ok::<(), UiError>(())
    })();
    if let Err(error) = build_result {
        let _ = tree.clear_owned(&app, "commands");
        return Err(error);
    }
    // SAFETY: NSApplication retains the installed main menu.
    let install_result = app.with(|app| {
        main.with(|main| unsafe {
            zpd_objc::msg_send!(app, zpd_objc::sel!("setMainMenu:"), ((main): zpd_objc::Id) => ())
        })
    });
    match install_result {
        Ok(Ok(())) => {}
        Ok(Err(_)) | Err(_) => {
            let _ = tree.clear_owned(&app, "commands");
            return Err(UiError::Closed);
        }
    }
    Ok(())
}

/// Builds an `NSMenu` Actor owned by `owner` from declarative items.
///
/// Items with an `id` and no role call `callback` with that id.
pub(crate) fn build_menu(
    tree: &ActorTree,
    owner: &ActorRef,
    title: &str,
    items: &[CommandItem],
    callback: &MenuCallback,
) -> Result<ActorRef, UiError> {
    let title = native::nsstring(title);
    let menu = native::alloc_init(zpd_objc::class!("NSMenu"));
    // SAFETY: The retained NSMenu copies its title.
    unsafe {
        zpd_objc::msg_send!(menu.as_ptr(), zpd_objc::sel!("setTitle:"), ((title.as_ptr()): Id) => ())
    };
    let menu = tree.insert_child(owner, menu).map_err(UiError::from)?;
    for item in items {
        add_item(tree, &menu, item, callback)?;
    }
    Ok(menu)
}

fn add_submenu(
    tree: &ActorTree,
    parent: &ActorRef,
    menu: &CommandMenu,
    callback: &MenuCallback,
) -> Result<(), UiError> {
    let container = native::alloc_init(zpd_objc::class!("NSMenuItem"));
    let title = native::nsstring(&menu.title);
    // SAFETY: The live parent NSMenu retains the container item.
    unsafe {
        zpd_objc::msg_send!(container.as_ptr(), zpd_objc::sel!("setTitle:"), ((title.as_ptr()): Id) => ());
        zpd_objc::msg_send!(parent.with(|id| id).map_err(UiError::from)?, zpd_objc::sel!("addItem:"), ((container.as_ptr()): Id) => ());
    }
    let container = tree
        .insert_child(parent, container)
        .map_err(UiError::from)?;
    let submenu = build_menu(tree, &container, &menu.title, &menu.items, callback)?;
    container
        .with(|container| {
            submenu.with(|submenu| {
                // SAFETY: The container item retains its submenu.
                unsafe {
                    zpd_objc::msg_send!(container, zpd_objc::sel!("setSubmenu:"), ((submenu): Id) => ())
                }
            })
        })
        .map_err(UiError::from)?
        .map_err(UiError::from)?;
    if let Some(role) = menu.role {
        let selector = match role {
            MenuRole::Services => zpd_objc::sel!("setServicesMenu:"),
            MenuRole::Windows => zpd_objc::sel!("setWindowsMenu:"),
            MenuRole::Help => zpd_objc::sel!("setHelpMenu:"),
        };
        submenu
            .with(|submenu| {
                // SAFETY: Each selector is an NSApplication setter taking an NSMenu.
                unsafe {
                    let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => Id);
                    zpd_objc::msg_send!(app, selector, ((submenu): Id) => ());
                }
            })
            .map_err(UiError::from)?;
    }
    Ok(())
}

fn add_item(
    tree: &ActorTree,
    menu: &ActorRef,
    item: &CommandItem,
    callback: &MenuCallback,
) -> Result<(), UiError> {
    let menu_id = menu.with(|id| id).map_err(UiError::from)?;
    if item.role == Some(CommandRole::Separator) {
        // SAFETY: separatorItem returns an autoreleased NSMenuItem retained here.
        let separator = unsafe {
            Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("NSMenuItem"), zpd_objc::sel!("separatorItem"), () => Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        // SAFETY: The live menu retains the separator.
        unsafe {
            zpd_objc::msg_send!(menu_id, zpd_objc::sel!("addItem:"), ((separator.as_ptr()): Id) => ())
        };
        tree.insert_child(menu, separator).map_err(UiError::from)?;
        return Ok(());
    }
    if let Some(submenu) = &item.submenu {
        let submenu = CommandMenu {
            title: item.title.clone(),
            ..submenu.clone()
        };
        return add_submenu(tree, menu, &submenu, callback);
    }
    let title = native::nsstring(&item.title);
    let key = native::nsstring(
        item.key
            .as_deref()
            .and_then(|v| v.chars().next())
            .map(|v| v.to_lowercase().to_string())
            .as_deref()
            .unwrap_or(""),
    );
    let custom = item.role.is_none() && item.id.is_some();
    // Disabled items lose their action so menu auto-enabling keeps them disabled.
    let selector = match (item.enabled, item.role) {
        (false, _) => zpd_objc::NIL,
        (true, Some(role)) => role.selector().unwrap_or(zpd_objc::NIL),
        (true, None) if custom => zpd_objc::sel!("invoke:"),
        (true, None) => zpd_objc::NIL,
    };
    // SAFETY: NSMenuItem's designated initializer copies its title and key.
    let native_item = unsafe {
        let allocated = zpd_objc::msg_send!(zpd_objc::class!("NSMenuItem"), zpd_objc::sel!("alloc"), () => Id);
        Strong::from_retained(zpd_objc::msg_send!(allocated, zpd_objc::sel!("initWithTitle:action:keyEquivalent:"), ((title.as_ptr()): Id, (selector): Id, (key.as_ptr()): Id) => Id))
    }
    .ok_or(UiError::NativeCreationFailed)?;
    let modifiers = item.modifiers.iter().fold(0_u64, |mask, value| {
        mask | match value {
            CommandModifier::Cmd => native::NS_EVENT_MODIFIER_FLAG_COMMAND,
            CommandModifier::Ctrl => native::NS_EVENT_MODIFIER_FLAG_CONTROL,
            CommandModifier::Alt => native::NS_EVENT_MODIFIER_FLAG_OPTION,
            CommandModifier::Shift => native::NS_EVENT_MODIFIER_FLAG_SHIFT,
        }
    });
    let state = if item.checked {
        native::NS_CONTROL_STATE_VALUE_ON
    } else {
        0
    };
    // SAFETY: The retained item and live menu use the declared ABI.
    unsafe {
        zpd_objc::msg_send!(native_item.as_ptr(), zpd_objc::sel!("setKeyEquivalentModifierMask:"), ((modifiers): u64) => ());
        zpd_objc::msg_send!(native_item.as_ptr(), zpd_objc::sel!("setState:"), ((state): native::Integer) => ());
        if let Some(role) = item.role {
            zpd_objc::msg_send!(native_item.as_ptr(), zpd_objc::sel!("setTag:"), ((role.tag()): native::Integer) => ());
            zpd_objc::msg_send!(native_item.as_ptr(), zpd_objc::sel!("setTarget:"), ((role.target()): Id) => ());
        }
        zpd_objc::msg_send!(menu_id, zpd_objc::sel!("addItem:"), ((native_item.as_ptr()): Id) => ());
    }
    let native_item = tree
        .insert_child(menu, native_item)
        .map_err(UiError::from)?;
    if custom && item.enabled {
        let id = item.id.clone().expect("custom items have ids");
        let callback = callback.clone();
        callback::install_action(&native_item, move |_| {
            let Ok(mut callback) = callback.try_borrow_mut() else {
                std::process::abort()
            };
            callback(&id);
        })?;
    }
    Ok(())
}
