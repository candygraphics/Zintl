//! A selectable list whose rows are arbitrary views.

use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement, KeyedElement};
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_backend_core::{ListStyle, ViewKind};
use zintl_ui_layout::{LayoutStyle, Size};

use crate::attach::attachments;
use crate::{Attachments, Context, Element, Event, EventKind, NativeView, RenderNode};

type SelectionAction = dyn for<'a> Fn(&mut Context<'a>, Option<&str>);

/// A row and the ID that selection reports for it.
#[derive(Clone)]
pub struct ListRow {
    id: String,
    content: ElementFactory<RenderNode>,
}

impl ListRow {
    pub fn new<V>(id: impl Into<String>, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            id: id.into(),
            content: ElementFactory::new(content),
        }
    }
}

/// Rows under an optional header.
#[derive(Clone)]
pub struct ListSection {
    title: Option<String>,
    rows: Vec<ListRow>,
}

impl ListSection {
    pub fn new(rows: impl IntoIterator<Item = ListRow>) -> Self {
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
/// Row IDs must be unique across the list. Rows are keyed by ID, so their
/// views keep state when rows move.
#[derive(Clone)]
pub struct List {
    sections: Vec<ListSection>,
    style: ListStyle,
    selection: Option<Store<Option<String>>>,
    action: Option<Rc<SelectionAction>>,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl List {
    pub fn new(sections: impl IntoIterator<Item = ListSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            style: ListStyle::default(),
            selection: None,
            action: None,
            layout: LayoutStyle::fill(Size::new(0.0, 0.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn style(mut self, style: ListStyle) -> Self {
        self.style = style;
        self
    }

    /// Reads and writes the selected row ID.
    pub fn bind(mut self, selection: Store<Option<String>>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Runs after the user changes the selection, once the bound Store is updated.
    pub fn on_select(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, Option<&str>) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.layout.minimum_size = size;
        self
    }

    fn route(&self, element: Element<RenderNode>) -> Element<RenderNode> {
        let store = self.selection;
        let action = self.action.clone();
        if store.is_none() && action.is_none() {
            return element;
        }
        element.on_event(EventKind::ListSelectionChanged, move |cx, event| {
            if let Event::ListSelectionChanged { id } = event {
                if let Some(store) = store {
                    cx.update(store, |selection| *selection = id.clone());
                }
                if let Some(action) = &action {
                    action(cx, id.as_deref());
                }
            }
        })
    }
}

impl NativeView for List {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let sections = self
            .sections
            .iter()
            .map(|section| zintl_ui_backend_core::ListSection {
                title: section.title.clone(),
                ids: section.rows.iter().map(|row| row.id.clone()).collect(),
            })
            .collect();
        let rows = self
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .map(|row| row.content.clone().key(row.id.clone()).into_element());
        let element = Element::node(RenderNode::View {
            kind: ViewKind::List {
                sections,
                selected: self.selection.and_then(|store| cx.get(store).clone()),
                style: self.style,
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: true,
            title_bar_interactive: false,
        })
        .with_children(rows);
        self.route(element)
    }
}

impl View for List {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
