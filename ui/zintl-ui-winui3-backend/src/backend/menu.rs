use zintl_ui_appkit_backend::{CommandItem, CommandModifier, CommandRole, CommandSet, MenuRole};
use zpd_winui3::{
    KeyboardAccelerator, MenuBar, MenuBarItem, MenuFlyoutItem, MenuFlyoutItemBase,
    MenuFlyoutSubItem, MenuItemRole, VirtualKeyModifiers,
};

/// Converts declared commands into WinUI menus.
pub(super) struct Menus;

impl Menus {
    /// Ends the application; reported instead of a declared id.
    pub(super) const QUIT: &str = "zintl.winui3.quit";
    /// Closes the window whose menu bar was used.
    pub(super) const CLOSE: &str = "zintl.winui3.close";

    /// Builds a window menu bar; App menu items join the File menu.
    pub(super) fn menu_bar(commands: &CommandSet) -> MenuBar {
        let mut menus = commands
            .menus
            .iter()
            .filter(|menu| menu.role != Some(MenuRole::Services))
            .map(|menu| MenuBarItem {
                title: menu.title.clone(),
                items: Self::items(&menu.items),
            })
            .collect::<Vec<_>>();
        let app_items = commands
            .app_menu
            .as_ref()
            .map(|menu| Self::items(&menu.items))
            .unwrap_or_default();
        if app_items
            .iter()
            .any(|item| !matches!(item, MenuFlyoutItemBase::Separator))
        {
            let file = match menus.iter().position(|menu| menu.title == "File") {
                Some(index) => &mut menus[index],
                None => {
                    menus.insert(
                        0,
                        MenuBarItem {
                            title: "File".into(),
                            items: Vec::new(),
                        },
                    );
                    &mut menus[0]
                }
            };
            if !file.items.is_empty() {
                file.items.push(MenuFlyoutItemBase::Separator);
            }
            file.items.extend(app_items);
            file.items = Self::tidy(std::mem::take(&mut file.items));
        }
        menus.retain(|menu| !menu.items.is_empty());
        MenuBar { items: menus }
    }

    /// Converts menu items, dropping actions that have no Windows equivalent.
    pub(super) fn items(items: &[CommandItem]) -> Vec<MenuFlyoutItemBase> {
        let converted = items.iter().filter_map(Self::item).collect();
        Self::tidy(converted)
    }

    fn item(item: &CommandItem) -> Option<MenuFlyoutItemBase> {
        if item.role == Some(CommandRole::Separator) {
            return Some(MenuFlyoutItemBase::Separator);
        }
        if let Some(submenu) = &item.submenu {
            if submenu.role == Some(MenuRole::Services) {
                return None;
            }
            let items = Self::items(&submenu.items);
            return (!items.is_empty()).then(|| {
                MenuFlyoutItemBase::SubItem(MenuFlyoutSubItem {
                    text: item.title.clone(),
                    is_enabled: item.enabled,
                    items,
                })
            });
        }
        let (id, role) = match item.role {
            None => (item.id.clone(), MenuItemRole::None),
            Some(CommandRole::Quit) => (Some(Self::QUIT.into()), MenuItemRole::None),
            Some(CommandRole::Close) => (Some(Self::CLOSE.into()), MenuItemRole::None),
            Some(CommandRole::Cut) => (None, MenuItemRole::Cut),
            Some(CommandRole::Copy) => (None, MenuItemRole::Copy),
            Some(CommandRole::Paste) => (None, MenuItemRole::Paste),
            Some(CommandRole::SelectAll) => (None, MenuItemRole::SelectAll),
            Some(CommandRole::Undo) => (None, MenuItemRole::Undo),
            Some(CommandRole::Redo) => (None, MenuItemRole::Redo),
            Some(CommandRole::Delete) => (None, MenuItemRole::Delete),
            // macOS window and app management have no Windows counterpart.
            Some(_) => return None,
        };
        let enabled = item.enabled && (id.is_some() || role != MenuItemRole::None);
        Some(MenuFlyoutItemBase::Item(MenuFlyoutItem {
            id: id.unwrap_or_default(),
            text: Self::title(item),
            is_enabled: enabled,
            keyboard_accelerator: Self::accelerator(item),
            role,
        }))
    }

    /// Uses Windows wording for standard items.
    fn title(item: &CommandItem) -> String {
        match item.role {
            Some(CommandRole::Quit) => "Exit".into(),
            _ => item.title.clone(),
        }
    }

    /// Maps a shortcut; Command becomes Control, and unsupported keys are dropped.
    fn accelerator(item: &CommandItem) -> Option<KeyboardAccelerator> {
        let key = item.key.as_deref()?;
        let mut characters = key.chars();
        let character = characters.next()?;
        let supported =
            characters.next().is_none() && (character.is_ascii_alphanumeric() || character == ',');
        if !supported {
            return None;
        }
        let mut modifiers = VirtualKeyModifiers::NONE;
        for modifier in &item.modifiers {
            modifiers = modifiers
                | match modifier {
                    CommandModifier::Cmd | CommandModifier::Ctrl => VirtualKeyModifiers::CONTROL,
                    CommandModifier::Alt => VirtualKeyModifiers::MENU,
                    CommandModifier::Shift => VirtualKeyModifiers::SHIFT,
                };
        }
        Some(KeyboardAccelerator {
            key: key.to_ascii_uppercase(),
            modifiers,
        })
    }

    /// Removes leading, trailing, and repeated separators.
    fn tidy(items: Vec<MenuFlyoutItemBase>) -> Vec<MenuFlyoutItemBase> {
        let mut result: Vec<MenuFlyoutItemBase> = Vec::with_capacity(items.len());
        for item in items {
            let separator = matches!(item, MenuFlyoutItemBase::Separator);
            let after_separator = result
                .last()
                .is_none_or(|last| matches!(last, MenuFlyoutItemBase::Separator));
            if !(separator && after_separator) {
                result.push(item);
            }
        }
        if matches!(result.last(), Some(MenuFlyoutItemBase::Separator)) {
            result.pop();
        }
        result
    }
}
