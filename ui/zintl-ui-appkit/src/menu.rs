//! Declarative menus for context menus and the application's main menu.
//!
//! Standard items are identified by [`StandardText`] and resolved through a
//! [`MenuLocalizer`] when the menu renders, so translations can be added
//! without changing menu declarations.

use std::rc::Rc;

use zintl_ui::element::IntoElement;
use zintl_ui::view::View;

use crate::{Context, Element, Event, EventKind, RenderNode};

pub(crate) type MenuAction = dyn for<'a> Fn(&mut Context<'a>);

/// Text owned by the standard menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandardText {
    About,
    Settings,
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    FileMenu,
    Close,
    EditMenu,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    ViewMenu,
    ToggleSidebar,
    ToggleFullScreen,
    WindowMenu,
    Minimize,
    Zoom,
    BringAllToFront,
    HelpMenu,
    Help,
}

impl StandardText {
    /// Returns a stable key for message catalogs.
    pub const fn key(self) -> &'static str {
        match self {
            Self::About => "menu.app.about",
            Self::Settings => "menu.app.settings",
            Self::Services => "menu.app.services",
            Self::Hide => "menu.app.hide",
            Self::HideOthers => "menu.app.hide-others",
            Self::ShowAll => "menu.app.show-all",
            Self::Quit => "menu.app.quit",
            Self::FileMenu => "menu.file",
            Self::Close => "menu.file.close",
            Self::EditMenu => "menu.edit",
            Self::Undo => "menu.edit.undo",
            Self::Redo => "menu.edit.redo",
            Self::Cut => "menu.edit.cut",
            Self::Copy => "menu.edit.copy",
            Self::Paste => "menu.edit.paste",
            Self::Delete => "menu.edit.delete",
            Self::SelectAll => "menu.edit.select-all",
            Self::ViewMenu => "menu.view",
            Self::ToggleSidebar => "menu.view.toggle-sidebar",
            Self::ToggleFullScreen => "menu.view.toggle-full-screen",
            Self::WindowMenu => "menu.window",
            Self::Minimize => "menu.window.minimize",
            Self::Zoom => "menu.window.zoom",
            Self::BringAllToFront => "menu.window.bring-all-to-front",
            Self::HelpMenu => "menu.help",
            Self::Help => "menu.help.app-help",
        }
    }
}

/// Supplies translated titles for standard menu text.
pub trait MenuLocalizer {
    /// Returns the title for `text`; `app_name` fills titles such as "About …".
    fn text(&self, text: StandardText, app_name: &str) -> String;
}

/// The default macOS English menu titles.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnglishMenuLocalizer;

impl MenuLocalizer for EnglishMenuLocalizer {
    fn text(&self, text: StandardText, app_name: &str) -> String {
        match text {
            StandardText::About => format!("About {app_name}"),
            StandardText::Settings => "Settings…".into(),
            StandardText::Services => "Services".into(),
            StandardText::Hide => format!("Hide {app_name}"),
            StandardText::HideOthers => "Hide Others".into(),
            StandardText::ShowAll => "Show All".into(),
            StandardText::Quit => format!("Quit {app_name}"),
            StandardText::FileMenu => "File".into(),
            StandardText::Close => "Close".into(),
            StandardText::EditMenu => "Edit".into(),
            StandardText::Undo => "Undo".into(),
            StandardText::Redo => "Redo".into(),
            StandardText::Cut => "Cut".into(),
            StandardText::Copy => "Copy".into(),
            StandardText::Paste => "Paste".into(),
            StandardText::Delete => "Delete".into(),
            StandardText::SelectAll => "Select All".into(),
            StandardText::ViewMenu => "View".into(),
            StandardText::ToggleSidebar => "Toggle Sidebar".into(),
            StandardText::ToggleFullScreen => "Toggle Full Screen".into(),
            StandardText::WindowMenu => "Window".into(),
            StandardText::Minimize => "Minimize".into(),
            StandardText::Zoom => "Zoom".into(),
            StandardText::BringAllToFront => "Bring All to Front".into(),
            StandardText::HelpMenu => "Help".into(),
            StandardText::Help => format!("{app_name} Help"),
        }
    }
}

/// The macOS Japanese menu titles.
#[derive(Clone, Copy, Debug, Default)]
pub struct JapaneseMenuLocalizer;

impl MenuLocalizer for JapaneseMenuLocalizer {
    fn text(&self, text: StandardText, app_name: &str) -> String {
        match text {
            StandardText::About => format!("{app_name}について"),
            StandardText::Settings => "設定…".into(),
            StandardText::Services => "サービス".into(),
            StandardText::Hide => format!("{app_name}を非表示"),
            StandardText::HideOthers => "ほかを非表示".into(),
            StandardText::ShowAll => "すべてを表示".into(),
            StandardText::Quit => format!("{app_name}を終了"),
            StandardText::FileMenu => "ファイル".into(),
            StandardText::Close => "閉じる".into(),
            StandardText::EditMenu => "編集".into(),
            StandardText::Undo => "取り消す".into(),
            StandardText::Redo => "やり直す".into(),
            StandardText::Cut => "カット".into(),
            StandardText::Copy => "コピー".into(),
            StandardText::Paste => "ペースト".into(),
            StandardText::Delete => "削除".into(),
            StandardText::SelectAll => "すべてを選択".into(),
            StandardText::ViewMenu => "表示".into(),
            StandardText::ToggleSidebar => "サイドバーの表示を切り替え".into(),
            StandardText::ToggleFullScreen => "フルスクリーンの切り替え".into(),
            StandardText::WindowMenu => "ウインドウ".into(),
            StandardText::Minimize => "しまう".into(),
            StandardText::Zoom => "拡大／縮小".into(),
            StandardText::BringAllToFront => "すべてを手前に移動".into(),
            StandardText::HelpMenu => "ヘルプ".into(),
            StandardText::Help => format!("{app_name}ヘルプ"),
        }
    }
}

/// Actions AppKit delivers through the responder chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardAction {
    About,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    Close,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    ToggleSidebar,
    ToggleFullScreen,
    Minimize,
    Zoom,
    BringAllToFront,
    Help,
}

impl StandardAction {
    fn text(self) -> StandardText {
        match self {
            Self::About => StandardText::About,
            Self::Hide => StandardText::Hide,
            Self::HideOthers => StandardText::HideOthers,
            Self::ShowAll => StandardText::ShowAll,
            Self::Quit => StandardText::Quit,
            Self::Close => StandardText::Close,
            Self::Undo => StandardText::Undo,
            Self::Redo => StandardText::Redo,
            Self::Cut => StandardText::Cut,
            Self::Copy => StandardText::Copy,
            Self::Paste => StandardText::Paste,
            Self::Delete => StandardText::Delete,
            Self::SelectAll => StandardText::SelectAll,
            Self::ToggleSidebar => StandardText::ToggleSidebar,
            Self::ToggleFullScreen => StandardText::ToggleFullScreen,
            Self::Minimize => StandardText::Minimize,
            Self::Zoom => StandardText::Zoom,
            Self::BringAllToFront => StandardText::BringAllToFront,
            Self::Help => StandardText::Help,
        }
    }

    /// Returns the shortcut macOS apps conventionally use.
    fn shortcut(self) -> Option<Shortcut> {
        use Modifier::{Command, Control, Option as Alt, Shift};
        let (key, modifiers): (&str, &[Modifier]) = match self {
            Self::Hide => ("h", &[Command]),
            Self::HideOthers => ("h", &[Command, Alt]),
            Self::Quit => ("q", &[Command]),
            Self::Close => ("w", &[Command]),
            Self::Undo => ("z", &[Command]),
            Self::Redo => ("z", &[Command, Shift]),
            Self::Cut => ("x", &[Command]),
            Self::Copy => ("c", &[Command]),
            Self::Paste => ("v", &[Command]),
            Self::SelectAll => ("a", &[Command]),
            Self::ToggleSidebar => ("s", &[Command, Control]),
            Self::ToggleFullScreen => ("f", &[Command, Control]),
            Self::Minimize => ("m", &[Command]),
            Self::Help => ("?", &[Command]),
            Self::About | Self::ShowAll | Self::Delete | Self::Zoom | Self::BringAllToFront => {
                return None;
            }
        };
        Some(Shortcut::new(key, modifiers.iter().copied()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    Command,
    Shift,
    Option,
    Control,
}

/// A key equivalent such as ⌘N.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub key: String,
    pub modifiers: Vec<Modifier>,
}

impl Shortcut {
    pub fn new(key: impl Into<String>, modifiers: impl IntoIterator<Item = Modifier>) -> Self {
        Self {
            key: key.into(),
            modifiers: modifiers.into_iter().collect(),
        }
    }

    /// Creates a ⌘-key shortcut.
    pub fn command(key: impl Into<String>) -> Self {
        Self::new(key, [Modifier::Command])
    }

    pub fn shift(mut self) -> Self {
        self.modifiers.push(Modifier::Shift);
        self
    }

    pub fn option(mut self) -> Self {
        self.modifiers.push(Modifier::Option);
        self
    }

    pub fn control(mut self) -> Self {
        self.modifiers.push(Modifier::Control);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MenuText {
    Literal(String),
    Standard(StandardText),
}

impl MenuText {
    fn resolve(&self, localizer: &dyn MenuLocalizer, app_name: &str) -> String {
        match self {
            Self::Literal(text) => text.clone(),
            Self::Standard(text) => localizer.text(*text, app_name),
        }
    }
}

#[derive(Clone)]
enum MenuItemKind {
    Action {
        id: Option<String>,
        title: MenuText,
        standard: Option<StandardAction>,
        shortcut: Option<Shortcut>,
        enabled: bool,
        checked: bool,
        action: Option<Rc<MenuAction>>,
    },
    Separator,
    Submenu(Menu),
}

/// An item of a [`Menu`].
#[derive(Clone)]
pub struct MenuItem(MenuItemKind);

impl MenuItem {
    /// Creates an item; it stays disabled until it has an action.
    pub fn new(title: impl Into<String>) -> Self {
        Self::with_title(MenuText::Literal(title.into()), None)
    }

    /// Creates an item handled by the responder chain, such as Copy.
    pub fn standard(action: StandardAction) -> Self {
        let mut item = Self::with_title(MenuText::Standard(action.text()), Some(action));
        if let MenuItemKind::Action { shortcut, .. } = &mut item.0 {
            *shortcut = action.shortcut();
        }
        item
    }

    /// Creates the "Settings…" item with its ⌘, shortcut.
    pub fn settings(action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        Self::with_title(MenuText::Standard(StandardText::Settings), None)
            .shortcut(Shortcut::command(","))
            .on_select(action)
    }

    pub fn separator() -> Self {
        Self(MenuItemKind::Separator)
    }

    pub fn submenu(menu: Menu) -> Self {
        Self(MenuItemKind::Submenu(menu))
    }

    fn with_title(title: MenuText, standard: Option<StandardAction>) -> Self {
        Self(MenuItemKind::Action {
            id: None,
            title,
            standard,
            shortcut: None,
            enabled: true,
            checked: false,
            action: None,
        })
    }

    fn update(mut self, update: impl FnOnce(&mut MenuItemKind)) -> Self {
        update(&mut self.0);
        self
    }

    /// Sets a stable ID; otherwise one is derived from the item's position.
    pub fn id(self, value: impl Into<String>) -> Self {
        let value = value.into();
        self.update(|kind| {
            if let MenuItemKind::Action { id, .. } = kind {
                *id = Some(value);
            }
        })
    }

    pub fn shortcut(self, value: Shortcut) -> Self {
        self.update(|kind| {
            if let MenuItemKind::Action { shortcut, .. } = kind {
                *shortcut = Some(value);
            }
        })
    }

    pub fn enabled(self, value: bool) -> Self {
        self.update(|kind| {
            if let MenuItemKind::Action { enabled, .. } = kind {
                *enabled = value;
            }
        })
    }

    /// Shows a checkmark before the title.
    pub fn checked(self, value: bool) -> Self {
        self.update(|kind| {
            if let MenuItemKind::Action { checked, .. } = kind {
                *checked = value;
            }
        })
    }

    pub fn on_select(self, value: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.update(|kind| {
            if let MenuItemKind::Action { action, .. } = kind {
                *action = Some(Rc::new(value));
            }
        })
    }
}

/// Menus that AppKit fills or decorates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuRole {
    Services,
    Windows,
    Help,
}

/// A titled list of menu items.
#[derive(Clone)]
pub struct Menu {
    title: MenuText,
    items: Vec<MenuItem>,
    role: Option<MenuRole>,
    localizer: Option<Rc<dyn MenuLocalizer>>,
}

impl Menu {
    pub fn new(title: impl Into<String>, items: impl IntoIterator<Item = MenuItem>) -> Self {
        Self::with_title(MenuText::Literal(title.into()), items)
    }

    fn with_title(title: MenuText, items: impl IntoIterator<Item = MenuItem>) -> Self {
        Self {
            title,
            items: items.into_iter().collect(),
            role: None,
            localizer: None,
        }
    }

    /// Creates an untitled menu, such as a context menu.
    pub fn items(items: impl IntoIterator<Item = MenuItem>) -> Self {
        Self::new("", items)
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Translates standard items in this menu.
    pub fn localizer(mut self, localizer: impl MenuLocalizer + 'static) -> Self {
        self.localizer = Some(Rc::new(localizer));
        self
    }

    fn role(mut self, role: MenuRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Renders this menu as a context menu for the parent view.
    pub(crate) fn context_element(&self) -> Element<RenderNode> {
        let mut actions = MenuActions::default();
        let menu = self.resolve(&EnglishMenuLocalizer, "", "context", &mut actions);
        actions.route(Element::node(RenderNode::NSMenu { menu }))
    }

    /// Resolves titles and assigns IDs to custom actions.
    pub(crate) fn resolve(
        &self,
        localizer: &dyn MenuLocalizer,
        app_name: &str,
        path: &str,
        actions: &mut MenuActions,
    ) -> MenuState {
        let localizer = self.localizer.as_deref().unwrap_or(localizer);
        MenuState {
            title: self.title.resolve(localizer, app_name),
            role: self.role,
            items: self
                .items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let path = format!("{path}.{index}");
                    match &item.0 {
                        MenuItemKind::Separator => MenuItemState::Separator,
                        MenuItemKind::Submenu(menu) => MenuItemState::Submenu(
                            menu.resolve(localizer, app_name, &path, actions),
                        ),
                        MenuItemKind::Action {
                            id,
                            title,
                            standard,
                            shortcut,
                            enabled,
                            checked,
                            action,
                        } => {
                            let id = action.as_ref().map(|action| {
                                let id = id.clone().unwrap_or(path);
                                actions.0.push((id.clone(), action.clone()));
                                id
                            });
                            MenuItemState::Action {
                                id,
                                title: title.resolve(localizer, app_name),
                                standard: *standard,
                                shortcut: shortcut.clone(),
                                enabled: *enabled && (action.is_some() || standard.is_some()),
                                checked: *checked,
                            }
                        }
                    }
                })
                .collect(),
        }
    }
}

impl<const N: usize> From<[MenuItem; N]> for Menu {
    fn from(items: [MenuItem; N]) -> Self {
        Self::items(items)
    }
}

impl From<Vec<MenuItem>> for Menu {
    fn from(items: Vec<MenuItem>) -> Self {
        Self::items(items)
    }
}

/// Actions collected while resolving menus, keyed by item ID.
#[derive(Clone, Default)]
pub(crate) struct MenuActions(Vec<(String, Rc<MenuAction>)>);

impl MenuActions {
    /// Routes item selection events on `element` to the collected actions.
    pub(crate) fn route(self, element: Element<RenderNode>) -> Element<RenderNode> {
        if self.0.is_empty() {
            return element;
        }
        element.on_event(EventKind::MenuItemSelected, move |cx, event| {
            if let Event::MenuItemSelected { id } = event
                && let Some((_, action)) = self.0.iter().find(|(candidate, _)| *candidate == id)
            {
                action(cx);
            }
        })
    }
}

/// Rendered menu data without actions.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuState {
    pub title: String,
    pub items: Vec<MenuItemState>,
    pub role: Option<MenuRole>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MenuItemState {
    Action {
        /// Present for custom actions; selection reports this ID.
        id: Option<String>,
        title: String,
        standard: Option<StandardAction>,
        shortcut: Option<Shortcut>,
        enabled: bool,
        checked: bool,
    },
    Separator,
    Submenu(MenuState),
}

/// The standard menus of a macOS app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardMenu {
    App,
    File,
    Edit,
    View,
    Window,
    Help,
}

/// The application's main menu bar.
///
/// Declare it at the root next to windows. Standard items use the
/// responder chain, so AppKit enables them for the focused control.
#[derive(Clone)]
pub struct MainMenu {
    app_name: Option<String>,
    localizer: Rc<dyn MenuLocalizer>,
    menus: Vec<StandardMenu>,
    extensions: Vec<(StandardMenu, MenuItem)>,
    custom: Vec<Menu>,
}

impl MainMenu {
    /// Creates the App, File, Edit, View, Window, and Help menus.
    pub fn standard() -> Self {
        Self {
            app_name: None,
            localizer: Rc::new(EnglishMenuLocalizer),
            menus: vec![
                StandardMenu::App,
                StandardMenu::File,
                StandardMenu::Edit,
                StandardMenu::View,
                StandardMenu::Window,
                StandardMenu::Help,
            ],
            extensions: Vec::new(),
            custom: Vec::new(),
        }
    }

    /// Creates only the App menu followed by `menus`.
    pub fn new(menus: impl IntoIterator<Item = Menu>) -> Self {
        Self {
            menus: vec![StandardMenu::App],
            custom: menus.into_iter().collect(),
            ..Self::standard()
        }
    }

    /// Sets the name used by About, Hide, Quit, and Help; defaults to the executable name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    pub fn localizer(mut self, localizer: impl MenuLocalizer + 'static) -> Self {
        self.localizer = Rc::new(localizer);
        self
    }

    /// Adds "Settings…" to the App menu.
    pub fn settings(self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.extend(StandardMenu::App, [MenuItem::settings(action)])
    }

    /// Adds items to a standard menu at its conventional position.
    pub fn extend(mut self, menu: StandardMenu, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.extensions
            .extend(items.into_iter().map(|item| (menu, item)));
        self
    }

    /// Adds a custom menu before the Window menu.
    pub fn menu(mut self, menu: Menu) -> Self {
        self.custom.push(menu);
        self
    }

    /// Removes a standard menu other than the App menu.
    pub fn without(mut self, menu: StandardMenu) -> Self {
        if menu != StandardMenu::App {
            self.menus.retain(|candidate| *candidate != menu);
        }
        self
    }

    fn app_name_or_default(&self) -> String {
        self.app_name.clone().unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| Some(path.file_stem()?.to_string_lossy().into_owned()))
                .unwrap_or_default()
        })
    }

    fn extensions(&self, menu: StandardMenu) -> Vec<MenuItem> {
        self.extensions
            .iter()
            .filter(|(target, _)| *target == menu)
            .map(|(_, item)| item.clone())
            .collect()
    }

    /// Builds one standard menu, placing extensions where macOS apps expect them.
    fn standard_menu(&self, menu: StandardMenu) -> Menu {
        use StandardAction as Action;
        let extra = self.extensions(menu);
        let separated = |items: Vec<MenuItem>| -> Vec<MenuItem> {
            if items.is_empty() {
                items
            } else {
                items.into_iter().chain([MenuItem::separator()]).collect()
            }
        };
        let (title, items, role) = match menu {
            StandardMenu::App => {
                let mut items = vec![MenuItem::standard(Action::About), MenuItem::separator()];
                items.extend(separated(extra));
                items.extend([
                    MenuItem::submenu(
                        Menu::with_title(MenuText::Standard(StandardText::Services), [])
                            .role(MenuRole::Services),
                    ),
                    MenuItem::separator(),
                    MenuItem::standard(Action::Hide),
                    MenuItem::standard(Action::HideOthers),
                    MenuItem::standard(Action::ShowAll),
                    MenuItem::separator(),
                    MenuItem::standard(Action::Quit),
                ]);
                (MenuText::Literal(String::new()), items, None)
            }
            StandardMenu::File => {
                let mut items = separated(extra);
                items.push(MenuItem::standard(Action::Close));
                (MenuText::Standard(StandardText::FileMenu), items, None)
            }
            StandardMenu::Edit => {
                let mut items = vec![
                    MenuItem::standard(Action::Undo),
                    MenuItem::standard(Action::Redo),
                    MenuItem::separator(),
                    MenuItem::standard(Action::Cut),
                    MenuItem::standard(Action::Copy),
                    MenuItem::standard(Action::Paste),
                    MenuItem::standard(Action::Delete),
                    MenuItem::standard(Action::SelectAll),
                ];
                if !extra.is_empty() {
                    items.push(MenuItem::separator());
                    items.extend(extra);
                }
                (MenuText::Standard(StandardText::EditMenu), items, None)
            }
            StandardMenu::View => {
                let mut items = separated(extra);
                items.extend([
                    MenuItem::standard(Action::ToggleSidebar),
                    MenuItem::standard(Action::ToggleFullScreen),
                ]);
                (MenuText::Standard(StandardText::ViewMenu), items, None)
            }
            StandardMenu::Window => {
                let mut items = vec![
                    MenuItem::standard(Action::Minimize),
                    MenuItem::standard(Action::Zoom),
                    MenuItem::separator(),
                ];
                items.extend(separated(extra));
                items.push(MenuItem::standard(Action::BringAllToFront));
                (
                    MenuText::Standard(StandardText::WindowMenu),
                    items,
                    Some(MenuRole::Windows),
                )
            }
            StandardMenu::Help => {
                let mut items = vec![MenuItem::standard(Action::Help)];
                items.extend(extra);
                (
                    MenuText::Standard(StandardText::HelpMenu),
                    items,
                    Some(MenuRole::Help),
                )
            }
        };
        Menu {
            title,
            items,
            role,
            localizer: None,
        }
    }

    /// Resolves the menu bar; the first menu is the App menu.
    pub(crate) fn resolve(&self, actions: &mut MenuActions) -> Vec<MenuState> {
        let app_name = self.app_name_or_default();
        let localizer = self.localizer.as_ref();
        let window_index = self
            .menus
            .iter()
            .position(|menu| matches!(menu, StandardMenu::Window | StandardMenu::Help))
            .unwrap_or(self.menus.len());
        let (leading, trailing) = self.menus.split_at(window_index);
        leading
            .iter()
            .map(|menu| self.standard_menu(*menu))
            .chain(self.custom.iter().cloned())
            .chain(trailing.iter().map(|menu| self.standard_menu(*menu)))
            .enumerate()
            .map(|(index, menu)| menu.resolve(localizer, &app_name, &format!("m{index}"), actions))
            .collect()
    }
}

impl Default for MainMenu {
    fn default() -> Self {
        Self::standard()
    }
}

impl View for MainMenu {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        let mut actions = MenuActions::default();
        let menus = self.resolve(&mut actions);
        actions.route(Element::node(RenderNode::NSMainMenu { menus }))
    }
}

mod backend {
    use super::*;
    use zintl_ui_appkit_backend as native;

    impl StandardAction {
        fn backend(self) -> native::CommandRole {
            match self {
                Self::About => native::CommandRole::About,
                Self::Hide => native::CommandRole::Hide,
                Self::HideOthers => native::CommandRole::HideOthers,
                Self::ShowAll => native::CommandRole::ShowAll,
                Self::Quit => native::CommandRole::Quit,
                Self::Close => native::CommandRole::Close,
                Self::Undo => native::CommandRole::Undo,
                Self::Redo => native::CommandRole::Redo,
                Self::Cut => native::CommandRole::Cut,
                Self::Copy => native::CommandRole::Copy,
                Self::Paste => native::CommandRole::Paste,
                Self::Delete => native::CommandRole::Delete,
                Self::SelectAll => native::CommandRole::SelectAll,
                Self::ToggleSidebar => native::CommandRole::ToggleSidebar,
                Self::ToggleFullScreen => native::CommandRole::ToggleFullScreen,
                Self::Minimize => native::CommandRole::Minimize,
                Self::Zoom => native::CommandRole::Zoom,
                Self::BringAllToFront => native::CommandRole::BringAllToFront,
                Self::Help => native::CommandRole::Help,
            }
        }
    }

    impl MenuState {
        pub(crate) fn backend(&self) -> native::CommandMenu {
            native::CommandMenu {
                title: self.title.clone(),
                items: MenuItemState::backend_items(&self.items),
                role: self.role.map(|role| match role {
                    MenuRole::Services => native::MenuRole::Services,
                    MenuRole::Windows => native::MenuRole::Windows,
                    MenuRole::Help => native::MenuRole::Help,
                }),
            }
        }

        /// Converts a menu bar whose first menu is the App menu.
        pub(crate) fn backend_menu_bar(menus: &[Self]) -> native::CommandSet {
            let mut menus = menus.iter().map(Self::backend);
            native::CommandSet {
                app_menu: menus
                    .next()
                    .map(|menu| native::WindowAppMenu { items: menu.items }),
                menus: menus.collect(),
            }
        }
    }

    impl MenuItemState {
        pub(crate) fn backend_items(items: &[Self]) -> Vec<native::CommandItem> {
            items.iter().map(Self::backend).collect()
        }

        fn backend(&self) -> native::CommandItem {
            match self {
                Self::Separator => native::CommandItem::separator(),
                Self::Submenu(menu) => native::CommandItem {
                    submenu: Some(menu.backend()),
                    ..native::CommandItem::new(menu.title.clone())
                },
                Self::Action {
                    id,
                    title,
                    standard,
                    shortcut,
                    enabled,
                    checked,
                } => native::CommandItem {
                    id: id.clone(),
                    role: standard.map(StandardAction::backend),
                    key: shortcut.as_ref().map(|shortcut| shortcut.key.clone()),
                    modifiers: shortcut
                        .iter()
                        .flat_map(|shortcut| &shortcut.modifiers)
                        .map(|modifier| match modifier {
                            Modifier::Command => native::CommandModifier::Cmd,
                            Modifier::Shift => native::CommandModifier::Shift,
                            Modifier::Option => native::CommandModifier::Alt,
                            Modifier::Control => native::CommandModifier::Ctrl,
                        })
                        .collect(),
                    enabled: *enabled,
                    checked: *checked,
                    ..native::CommandItem::new(title.clone())
                },
            }
        }
    }
}
