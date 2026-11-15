use zintl_ui_backend_core::{
    CommandItem, CommandModifier, CommandRole, CommandSet, MenuRole, NamedKey, ShortcutKey,
};
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
            Some(CommandRole::Bold) => (None, MenuItemRole::Bold),
            Some(CommandRole::Italic) => (None, MenuItemRole::Italic),
            Some(CommandRole::Underline) => (None, MenuItemRole::Underline),
            Some(CommandRole::TitleStyle) => (None, MenuItemRole::TitleStyle),
            Some(CommandRole::HeadingStyle) => (None, MenuItemRole::HeadingStyle),
            Some(CommandRole::SubheadingStyle) => (None, MenuItemRole::SubheadingStyle),
            Some(CommandRole::BodyStyle) => (None, MenuItemRole::BodyStyle),
            Some(CommandRole::MonospacedStyle) => (None, MenuItemRole::MonospacedStyle),
            Some(CommandRole::BulletedList) => (None, MenuItemRole::BulletedList),
            Some(CommandRole::DashedList) => (None, MenuItemRole::DashedList),
            Some(CommandRole::NumberedList) => (None, MenuItemRole::NumberedList),
            Some(CommandRole::Checklist) => (None, MenuItemRole::Checklist),
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

    /// Maps a shortcut; Command becomes Control.
    ///
    /// Only letters, digits, and named keys are kept: WinUI fails while
    /// opening a menu whose accelerator uses an OEM key such as the comma.
    fn accelerator(item: &CommandItem) -> Option<KeyboardAccelerator> {
        let key = match item.key.as_ref()? {
            ShortcutKey::Character(key) => {
                let mut characters = key.chars();
                let character = characters.next()?;
                if characters.next().is_some() || !character.is_ascii_alphanumeric() {
                    return None;
                }
                key.to_ascii_uppercase()
            }
            ShortcutKey::Named(key) => Self::virtual_key(*key)?.to_owned(),
        };
        let mut modifiers = VirtualKeyModifiers::NONE;
        for modifier in &item.modifiers {
            modifiers = modifiers
                | match modifier {
                    CommandModifier::Cmd | CommandModifier::Ctrl => VirtualKeyModifiers::CONTROL,
                    CommandModifier::Alt => VirtualKeyModifiers::MENU,
                    CommandModifier::Shift => VirtualKeyModifiers::SHIFT,
                };
        }
        Some(KeyboardAccelerator { key, modifiers })
    }

    /// Returns the name the support library maps to a `VirtualKey`.
    fn virtual_key(key: NamedKey) -> Option<&'static str> {
        Some(match key {
            NamedKey::Enter => "enter",
            NamedKey::Tab => "tab",
            NamedKey::Space => "space",
            NamedKey::Backspace => "backspace",
            NamedKey::Delete => "delete",
            NamedKey::Escape => "escape",
            NamedKey::ArrowLeft => "left",
            NamedKey::ArrowRight => "right",
            NamedKey::ArrowUp => "up",
            NamedKey::ArrowDown => "down",
            NamedKey::Home => "home",
            NamedKey::End => "end",
            NamedKey::PageUp => "pageup",
            NamedKey::PageDown => "pagedown",
            NamedKey::F(number @ 1..=24) => {
                const NAMES: [&str; 24] = [
                    "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12",
                    "f13", "f14", "f15", "f16", "f17", "f18", "f19", "f20", "f21", "f22", "f23",
                    "f24",
                ];
                NAMES[usize::from(number) - 1]
            }
            NamedKey::F(_) => return None,
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
