use std::rc::Rc;

use crate::{Context, Element, Event, EventKind, RenderNode, Store};

type SelectionAction = dyn for<'a> Fn(&mut Context<'a>, &str);

#[derive(Clone, Default)]
pub struct Sidebar {
    sections: Vec<SidebarSection>,
    selection: Option<Store<Option<String>>>,
    action: Option<Rc<SelectionAction>>,
    search: Option<Store<String>>,
    search_placeholder: Option<String>,
}

impl std::fmt::Debug for Sidebar {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Sidebar")
            .field("sections", &self.sections)
            .field("selection", &self.selection)
            .field("has_action", &self.action.is_some())
            .field("search", &self.search)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarSection {
    pub title: Option<String>,
    pub items: Vec<SidebarItem>,
}

impl SidebarSection {
    pub fn new(items: impl IntoIterator<Item = SidebarItem>) -> Self {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarItem {
    pub id: String,
    pub title: String,
    pub system_image: Option<String>,
}

impl SidebarItem {
    /// Item IDs should be unique across the sidebar.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            system_image: None,
        }
    }

    pub fn system_image(mut self, name: impl Into<String>) -> Self {
        self.system_image = Some(name.into());
        self
    }
}

impl Sidebar {
    pub fn new(sections: impl IntoIterator<Item = SidebarSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            selection: None,
            action: None,
            search: None,
            search_placeholder: None,
        }
    }

    pub fn bind(mut self, selection: Store<Option<String>>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Runs after the user changes the selection and the bound Store updates.
    pub fn on_select(mut self, action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static) -> Self {
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

    /// Describes the sidebar with its current selection and search text.
    pub(crate) fn state(&self, cx: &mut Context<'_>) -> zintl_ui_backend_core::Sidebar {
        zintl_ui_backend_core::Sidebar {
            sections: self
                .sections
                .iter()
                .map(|section| zintl_ui_backend_core::SidebarSection {
                    title: section.title.clone(),
                    items: section
                        .items
                        .iter()
                        .map(|item| zintl_ui_backend_core::SidebarItem {
                            id: item.id.clone(),
                            title: item.title.clone(),
                            system_image: item.system_image.clone(),
                        })
                        .collect(),
                })
                .collect(),
            selected_id: self.selection.and_then(|store| cx.get(store).clone()),
            search: self
                .search
                .map(|store| zintl_ui_backend_core::SidebarSearch {
                    value: cx.get(store).clone(),
                    placeholder: self.search_placeholder.clone(),
                }),
        }
    }

    pub(crate) fn route(&self, element: Element<RenderNode>) -> Element<RenderNode> {
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
        let sections = self.sections.clone();
        element.on_event(EventKind::SidebarSelectionChanged, move |cx, event| {
            if let Event::SidebarSelectionChanged { id } = event
                && sections
                    .iter()
                    .flat_map(|section| &section.items)
                    .any(|item| item.id == id)
            {
                if let Some(store) = store {
                    cx.update(store, |selection| *selection = Some(id.clone()));
                }
                if let Some(action) = &action {
                    action(cx, &id);
                }
            }
        })
    }
}
