//! A selectable list whose rows are arbitrary views.

use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement, KeyedElement};
use zintl_ui::store::Store;
use zintl_ui_backend_core::{ListStyle, ViewKind};
use zintl_ui_layout::{LayoutStyle, Size};

use crate::item_key::{ByItemKey, ItemIdFn, ItemIdSource, ItemKeys};
use crate::modifiers;
use crate::{Context, Element, Event, EventKind, NativeView, RenderNode, ViewModifiers};

type SelectionAction<K> = dyn for<'a> Fn(&mut Context<'a>, Option<&K>);

/// A row and the key that selection reports for it.
#[derive(Clone)]
pub struct ListRow<K = String> {
    key: K,
    content: ElementFactory<RenderNode>,
}

impl<K> ListRow<K> {
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

impl<K> ListSection<K> {
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
/// Row keys must be unique across the list. Rows are keyed by their item ID,
/// from [`ItemKey`](crate::ItemKey) or [`List::item_id`], so their views keep
/// state when rows move. A selected key that no row has shows nothing selected.
#[derive(Clone)]
pub struct List<K: 'static = String, I = ByItemKey> {
    sections: Vec<ListSection<K>>,
    style: ListStyle,
    selection: Option<Store<Option<K>>>,
    action: Option<Rc<SelectionAction<K>>>,
    item_id: I,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl<K: 'static> List<K> {
    pub fn new(sections: impl IntoIterator<Item = ListSection<K>>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            style: ListStyle::default(),
            selection: None,
            action: None,
            item_id: ByItemKey,
            layout: LayoutStyle::fill(Size::new(0.0, 0.0)),
            modifiers: ViewModifiers::default(),
        }
    }
}

impl<K: 'static, I> List<K, I> {
    /// Names rows by `item_id` instead of their key's `ItemKey` implementation.
    ///
    /// Use it for keys that cannot implement `ItemKey`, such as ID types from
    /// another crate. Different keys must return different strings.
    pub fn item_id(self, item_id: impl Fn(&K) -> String + 'static) -> List<K, ItemIdFn<K>> {
        List {
            sections: self.sections,
            style: self.style,
            selection: self.selection,
            action: self.action,
            item_id: ItemIdFn::new(item_id),
            layout: self.layout,
            modifiers: self.modifiers,
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
}

impl<K: Clone + PartialEq + 'static, I: ItemIdSource<K>> List<K, I> {
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

impl<K: Clone + PartialEq + 'static, I: ItemIdSource<K>> NativeView for List<K, I> {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let rows = self.sections.iter().flat_map(|section| &section.rows);
        let keys = ItemKeys::new(rows.clone().map(|row| &row.key), &self.item_id);
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

impl<K: Clone + PartialEq + 'static, I: ItemIdSource<K>> zintl_ui::view::View for List<K, I> {
    type Output = RenderNode;

    fn render(
        &self,
        cx: &mut Context<'_>,
    ) -> impl zintl_ui::element::IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
