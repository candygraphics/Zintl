//! A selectable list whose rows are arbitrary views.

use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement, KeyedElement};
use zintl_ui::store::Store;
use zintl_ui_backend_core::{ListStyle, ViewKind};
use zintl_ui_layout::{LayoutStyle, Size};

use crate::item_key::ItemKeys;
use crate::modifiers;
use crate::{Context, Element, Event, EventKind, ItemKey, NativeView, RenderNode, ViewModifiers};

type SelectionAction<K> = dyn for<'a> Fn(&mut Context<'a>, Option<&K>);

/// A row and the key that selection reports for it.
#[derive(Clone)]
pub struct ListRow<K = String> {
    key: K,
    content: ElementFactory<RenderNode>,
}

impl<K: ItemKey> ListRow<K> {
    pub fn new<V>(key: K, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            key,
            content: ElementFactory::new(content),
        }
    }
}

/// Rows under an optional header.
#[derive(Clone)]
pub struct ListSection<K = String> {
    title: Option<String>,
    rows: Vec<ListRow<K>>,
}

impl<K: ItemKey> ListSection<K> {
    pub fn new(rows: impl IntoIterator<Item = ListRow<K>>) -> Self {
        Self {
            title: None,
            rows: rows.into_iter().collect(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// A native list with single selection, sections, and keyboard navigation.
///
/// Row keys must be unique across the list. Rows are keyed by their
/// [`ItemKey::item_id`], so their views keep state when rows move. A selected
/// key that no row has shows nothing selected.
#[derive(Clone)]
pub struct List<K: ItemKey = String> {
    sections: Vec<ListSection<K>>,
    style: ListStyle,
    selection: Option<Store<Option<K>>>,
    action: Option<Rc<SelectionAction<K>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl<K: ItemKey> List<K> {
    pub fn new(sections: impl IntoIterator<Item = ListSection<K>>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            style: ListStyle::default(),
            selection: None,
            action: None,
            layout: LayoutStyle::fill(Size::new(0.0, 0.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn style(mut self, style: ListStyle) -> Self {
        self.style = style;
        self
    }

    /// Reads and writes the selected row's key.
    pub fn bind(mut self, selection: Store<Option<K>>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Runs after the user changes the selection, once the bound Store is updated.
    pub fn on_select(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, Option<&K>) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Reports selection changes for rows in `keys`, ignoring rows that are gone.
    fn route(&self, element: Element<RenderNode>, keys: ItemKeys<K>) -> Element<RenderNode> {
        let store = self.selection;
        let action = self.action.clone();
        if store.is_none() && action.is_none() {
            return element;
        }
        element.on_event(EventKind::ListSelectionChanged, move |cx, event| {
            let Event::ListSelectionChanged { id } = event else {
                return;
            };
            let Some(key) = keys.selection(id.as_deref()) else {
                return;
            };
            if let Some(store) = store {
                cx.update(store, |selection| *selection = key.clone());
            }
            if let Some(action) = &action {
                action(cx, key.as_ref());
            }
        })
    }
}

impl<K: ItemKey> NativeView for List<K> {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let rows = self.sections.iter().flat_map(|section| &section.rows);
        let keys = ItemKeys::new(rows.clone().map(|row| &row.key));
        let sections = {
            let mut ids = keys.ids();
            self.sections
                .iter()
                .map(|section| zintl_ui_backend_core::ListSection {
                    title: section.title.clone(),
                    ids: ids
                        .by_ref()
                        .take(section.rows.len())
                        .map(str::to_owned)
                        .collect(),
                })
                .collect()
        };
        let children = rows
            .zip(keys.ids())
            .map(|(row, id)| row.content.clone().key(id.to_owned()).into_element())
            .collect::<Vec<_>>();
        let selected = self.selection.and_then(|store| {
            let key = cx.get(store).as_ref()?;
            keys.id(key).map(str::to_owned)
        });
        let element = Element::node(RenderNode::View {
            kind: ViewKind::List {
                sections,
                selected,
                style: self.style,
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
        .with_children(children);
        self.route(element, keys)
    }
}

impl<K: ItemKey> zintl_ui::view::View for List<K> {
    type Output = RenderNode;

    fn render(
        &self,
        cx: &mut Context<'_>,
    ) -> impl zintl_ui::element::IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
