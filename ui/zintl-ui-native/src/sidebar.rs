use std::rc::Rc;

use crate::item_key::ItemKeys;
use crate::menu::MenuActions;
use crate::{
    Context, Element, EnglishMenuLocalizer, Event, EventKind, ItemKey, Menu, MenuItemState,
    RenderNode, Store,
};

type SelectionAction<K> = dyn for<'a> Fn(&mut Context<'a>, &K);

/// The window's source list, whose items are selected by key.
///
/// Item keys must be unique across the sidebar. A selected key that no item
/// has shows nothing selected.
#[derive(Clone)]
pub struct Sidebar<K: ItemKey = String> {
    sections: Vec<SidebarSection<K>>,
    selection: Option<Store<Option<K>>>,
    action: Option<Rc<SelectionAction<K>>>,
    search: Option<Store<String>>,
    search_placeholder: Option<String>,
}

impl<K: ItemKey> std::fmt::Debug for Sidebar<K> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Sidebar")
            .field("sections", &self.sections.len())
            .field("binds_selection", &self.selection.is_some())
            .field("has_action", &self.action.is_some())
            .field("search", &self.search)
            .finish()
    }
}

#[derive(Clone)]
pub struct SidebarSection<K = String> {
    title: Option<String>,
    items: Vec<SidebarItem<K>>,
}

impl<K: ItemKey> SidebarSection<K> {
    pub fn new(items: impl IntoIterator<Item = SidebarItem<K>>) -> Self {
        Self {
            title: None,
            items: items.into_iter().collect(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

#[derive(Clone)]
pub struct SidebarItem<K = String> {
    key: K,
    title: String,
    system_image: Option<String>,
    badge: Option<String>,
    menu: Option<Menu>,
}

impl<K: ItemKey> SidebarItem<K> {
    pub fn new(key: K, title: impl Into<String>) -> Self {
        Self {
            key,
            title: title.into(),
            system_image: None,
            badge: None,
            menu: None,
        }
    }
    pub fn system_image(mut self, name: impl Into<String>) -> Self {
        self.system_image = Some(name.into());
        self
    }

    /// Shows trailing text, such as a note count.
    pub fn badge(mut self, text: impl Into<String>) -> Self {
        self.badge = Some(text.into());
        self
    }

    /// Shows `menu` when the item is secondary-clicked.
    pub fn context_menu(mut self, menu: impl Into<Menu>) -> Self {
        self.menu = Some(menu.into());
        self
    }
}

impl<K: ItemKey> Sidebar<K> {
    pub fn new(sections: impl IntoIterator<Item = SidebarSection<K>>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            selection: None,
            action: None,
            search: None,
            search_placeholder: None,
        }
    }

    /// Reads and writes the selected item's key.
    pub fn bind(mut self, selection: Store<Option<K>>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Runs after the user changes the selection and the bound Store updates.
    pub fn on_select(mut self, action: impl for<'a> Fn(&mut Context<'a>, &K) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Shows a search field above the items that edits `query`.
    ///
    /// Filter the sections from the same Store to show matching items.
    pub fn searchable(mut self, query: Store<String>) -> Self {
        self.search = Some(query);
        self
    }

    pub fn search_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.search_placeholder = Some(placeholder.into());
        self
    }

    fn keys(&self) -> ItemKeys<K> {
        ItemKeys::new(
            self.sections
                .iter()
                .flat_map(|section| &section.items)
                .map(|item| &item.key),
        )
    }
}

/// A [`Sidebar`] of any key type, as the window holds it.
pub(crate) trait WindowSidebar {
    /// Describes the sidebar with its current selection and search text.
    ///
    /// Item menu actions are collected into `actions` for the window to route.
    fn state(
        &self,
        cx: &mut Context<'_>,
        actions: &mut MenuActions,
    ) -> zintl_ui_backend_core::Sidebar;

    /// Reports search and selection changes to the sidebar's Stores and action.
    fn route(&self, element: Element<RenderNode>) -> Element<RenderNode>;
}

impl<K: ItemKey> WindowSidebar for Sidebar<K> {
    fn state(
        &self,
        cx: &mut Context<'_>,
        actions: &mut MenuActions,
    ) -> zintl_ui_backend_core::Sidebar {
        let keys = self.keys();
        let mut ids = keys.ids();
        let sections = self
            .sections
            .iter()
            .map(|section| zintl_ui_backend_core::SidebarSection {
                title: section.title.clone(),
                items: section
                    .items
                    .iter()
                    .zip(ids.by_ref())
                    .map(|(item, id)| zintl_ui_backend_core::SidebarItem {
                        id: id.to_owned(),
                        title: item.title.clone(),
                        system_image: item.system_image.clone(),
                        badge: item.badge.clone(),
                        menu: item.menu.as_ref().map_or_else(Vec::new, |menu| {
                            let path = format!("sidebar.{id}");
                            let menu = menu.resolve(&EnglishMenuLocalizer, "", &path, actions);
                            MenuItemState::commands(&menu.items)
                        }),
                    })
                    .collect(),
            })
            .collect();
        let selected_id = self.selection.and_then(|store| {
            let key = cx.get(store).as_ref()?;
            keys.id(key).map(str::to_owned)
        });
        zintl_ui_backend_core::Sidebar {
            sections,
            selected_id,
            search: self
                .search
                .map(|store| zintl_ui_backend_core::SidebarSearch {
                    value: cx.get(store).clone(),
                    placeholder: self.search_placeholder.clone(),
                }),
        }
    }

    fn route(&self, element: Element<RenderNode>) -> Element<RenderNode> {
        let element = match self.search {
            Some(query) => element.on_event(EventKind::SidebarSearchChanged, move |cx, event| {
                if let Event::SidebarSearchChanged { value } = event {
                    cx.update(query, |current| *current = value);
                }
            }),
            None => element,
        };
        let store = self.selection;
        let action = self.action.clone();
        if store.is_none() && action.is_none() {
            return element;
        }
        let keys = self.keys();
        element.on_event(EventKind::SidebarSelectionChanged, move |cx, event| {
            // An item that is gone by the time the event arrives is ignored.
            if let Event::SidebarSelectionChanged { id } = event
                && let Some(key) = keys.key(&id)
            {
                if let Some(store) = store {
                    cx.update(store, |selection| *selection = Some(key.clone()));
                }
                if let Some(action) = &action {
                    action(cx, key);
                }
            }
        })
    }
}
