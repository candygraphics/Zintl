//! Window toolbar items with actions and a bound search field.

use std::rc::Rc;

use zintl_ui::store::Store;

use crate::{Context, Element, Event, EventKind, RenderNode};

type Action = dyn for<'a> Fn(&mut Context<'a>);

#[derive(Clone)]
enum Item {
    Button {
        id: String,
        label: String,
        system_image: String,
        enabled: bool,
        action: Option<Rc<Action>>,
    },
    Search {
        query: Store<String>,
        placeholder: Option<String>,
    },
    FlexibleSpace,
}

/// One toolbar item.
#[derive(Clone)]
pub struct ToolbarItem(Item);

impl ToolbarItem {
    /// A button showing an SF Symbol; `label` is its tooltip and accessibility label.
    pub fn button(
        id: impl Into<String>,
        label: impl Into<String>,
        system_image: impl Into<String>,
    ) -> Self {
        Self(Item::Button {
            id: id.into(),
            label: label.into(),
            system_image: system_image.into(),
            enabled: true,
            action: None,
        })
    }

    /// A search field that reads and writes `query`.
    pub fn search(query: Store<String>) -> Self {
        Self(Item::Search {
            query,
            placeholder: None,
        })
    }

    /// Space that pushes the items after it to the trailing edge.
    pub fn flexible_space() -> Self {
        Self(Item::FlexibleSpace)
    }

    /// Runs when the button is clicked.
    pub fn on_click(mut self, value: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        if let Item::Button { action, .. } = &mut self.0 {
            *action = Some(Rc::new(value));
        }
        self
    }

    pub fn enabled(mut self, value: bool) -> Self {
        if let Item::Button { enabled, .. } = &mut self.0 {
            *enabled = value;
        }
        self
    }

    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        if let Item::Search { placeholder, .. } = &mut self.0 {
            *placeholder = Some(value.into());
        }
        self
    }
}

/// A window toolbar. With a sidebar, the sidebar toggle comes first and the
/// items sit over the content.
#[derive(Clone)]
pub struct Toolbar {
    items: Vec<ToolbarItem>,
}

impl Toolbar {
    pub fn new(items: impl IntoIterator<Item = ToolbarItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    /// Describes the toolbar with the current search text.
    pub(crate) fn state(&self, cx: &mut Context<'_>) -> zintl_ui_backend_core::Toolbar {
        use zintl_ui_backend_core::ToolbarItem as Native;
        zintl_ui_backend_core::Toolbar {
            items: self
                .items
                .iter()
                .map(|item| match &item.0 {
                    Item::Button {
                        id,
                        label,
                        system_image,
                        enabled,
                        ..
                    } => Native::Button {
                        id: id.clone(),
                        label: label.clone(),
                        system_image: system_image.clone(),
                        enabled: *enabled,
                    },
                    Item::Search { query, placeholder } => Native::Search {
                        value: cx.get(*query).clone(),
                        placeholder: placeholder.clone(),
                    },
                    Item::FlexibleSpace => Native::FlexibleSpace,
                })
                .collect(),
        }
    }

    /// Routes button clicks to their actions and search edits to the query Store.
    pub(crate) fn route(&self, element: Element<RenderNode>) -> Element<RenderNode> {
        let buttons = self
            .items
            .iter()
            .filter_map(|item| match &item.0 {
                Item::Button {
                    id,
                    action: Some(action),
                    ..
                } => Some((id.clone(), action.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let query = self.items.iter().find_map(|item| match &item.0 {
            Item::Search { query, .. } => Some(*query),
            _ => None,
        });
        let element = element.on_event(EventKind::ToolbarItemClicked, move |cx, event| {
            if let Event::ToolbarItemClicked { id } = event
                && let Some((_, action)) = buttons.iter().find(|(button, _)| *button == id)
            {
                action(cx);
            }
        });
        match query {
            Some(query) => element.on_event(EventKind::ToolbarSearchChanged, move |cx, event| {
                if let Event::ToolbarSearchChanged { value } = event {
                    cx.update(query, |current| *current = value);
                }
            }),
            None => element,
        }
    }
}
