use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::store::{Store, Trigger};
use zintl_ui_backend_core::{ScrollAlignment, ScrollTarget, ViewKind};
use zintl_ui_layout::{Axis, LayoutStyle, Size};

use crate::modifiers;
use crate::{
    Change, ChangeRoute, Context, Element, Event, EventKind, NativeView, RenderNode, ScrollAxes,
    ScrollStatus, ViewId, ViewModifiers,
};

/// A native scroll view around its content.
#[derive(Clone)]
pub struct ScrollView {
    content: ElementFactory<RenderNode>,
    axes: ScrollAxes,
    store: Option<Store<ScrollStatus>>,
    target: Option<(Trigger<ViewId>, ScrollAlignment)>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl ScrollView {
    pub fn new<V>(content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            content: ElementFactory::new(content),
            axes: ScrollAxes::VERTICAL,
            store: None,
            target: None,
            layout: LayoutStyle::fill(Size::new(80.0, 80.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn axes(mut self, axes: ScrollAxes) -> Self {
        self.axes = axes;
        self
    }

    /// Saves the scroll position and restores it when the view is created again.
    pub fn bind_offset(mut self, store: Store<ScrollStatus>) -> Self {
        self.store = Some(store);
        self
    }

    /// Scrolls the descendant with the view ID `trigger` fires into view, each
    /// time it fires, even with the same ID.
    ///
    /// Give the rows IDs with their `id` builders, such as the row of a selected item.
    pub fn scroll_to(mut self, trigger: Trigger<ViewId>, alignment: ScrollAlignment) -> Self {
        self.target = Some((trigger, alignment));
        self
    }
}

impl NativeView for ScrollView {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::ScrollView {
                axes: self.axes,
                offset: self.store.map(|store| {
                    let status = *cx.get(store);
                    (status.x, status.y)
                }),
                reveal: self.target.and_then(|(trigger, alignment)| {
                    Some(ScrollTarget {
                        id: cx.get(trigger.store()).clone()?.into(),
                        alignment,
                        revision: cx.revision(trigger.store()),
                    })
                }),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
        .with_children([self.content.clone().into_element()]);
        let Some(store) = self.store else {
            return element;
        };
        element.on_event(EventKind::Scrolled, move |cx, event| {
            if let Event::Scrolled { x, y } = event {
                cx.update(store, |status| *status = ScrollStatus { x, y });
            }
        })
    }
}

native_view!(ScrollView);

/// One page of a [`TabView`], and the value that selects it.
#[derive(Clone)]
pub struct Tab<V = usize> {
    value: V,
    title: String,
    content: ElementFactory<RenderNode>,
}

impl<V> Tab<V> {
    pub fn new<C>(value: V, title: impl Into<String>, content: C) -> Self
    where
        C: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            value,
            title: title.into(),
            content: ElementFactory::new(content),
        }
    }
}

/// A native tab view that shows one page at a time.
///
/// The shown page comes from `bind`, then `selection`. Without either, or
/// with a value no tab has, the first page shows.
#[derive(Clone)]
pub struct TabView<V: Clone + PartialEq + 'static = usize> {
    items: Vec<Tab<V>>,
    selection: Option<V>,
    store: Option<Store<V>>,
    action: Option<Rc<Change<V>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl<V: Clone + PartialEq + 'static> TabView<V> {
    /// Inset between the tab view's content area and each page.
    const PAGE_PADDING: f32 = 12.0;

    pub fn new(items: impl IntoIterator<Item = Tab<V>>) -> Self {
        let items: Vec<Tab<V>> = items.into_iter().collect();
        debug_assert!(
            items
                .iter()
                .enumerate()
                .all(|(index, tab)| !items[..index].iter().any(|other| other.value == tab.value)),
            "tabs must have distinct values"
        );
        Self {
            items,
            selection: None,
            store: None,
            action: None,
            layout: LayoutStyle::fill(Size::new(200.0, 120.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    /// Shows the page with `value`.
    pub fn selection(mut self, value: V) -> Self {
        self.selection = Some(value);
        self
    }

    /// Reads and writes the selected page's value.
    pub fn bind(mut self, store: Store<V>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, &V) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Returns the index of the page to show.
    ///
    /// A native tab view always shows a page, so an unknown value shows the first.
    fn selected_index(&self, cx: &mut Context<'_>) -> usize {
        let value = match self.store {
            Some(store) => cx.get(store).clone(),
            None => match &self.selection {
                Some(value) => value.clone(),
                None => return 0,
            },
        };
        self.items
            .iter()
            .position(|item| item.value == value)
            .unwrap_or(0)
    }
}

impl<V: Clone + PartialEq + 'static> NativeView for TabView<V> {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let mut page_layout = LayoutStyle::stack(Axis::Vertical, 8.0);
        page_layout.padding = Self::PAGE_PADDING;
        let pages = self.items.iter().map(|item| {
            Element::node(RenderNode::View {
                kind: ViewKind::Container,
                layout: page_layout,
                id: None,
                enabled: true,
                title_bar_interactive: false,
                help: None,
                accessibility: Default::default(),
            })
            .with_children([item.content.clone().into_element()])
        });
        let element = Element::node(RenderNode::View {
            kind: ViewKind::TabView {
                titles: self.items.iter().map(|item| item.title.clone()).collect(),
                selected: self.selected_index(cx),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
        .with_children(pages);
        ChangeRoute::new(self.store, self.action.clone())
            .by_index(self.items.iter().map(|item| item.value.clone()))
            .route_selection(element)
    }
}

impl<V: Clone + PartialEq + 'static> zintl_ui::view::View for TabView<V> {
    type Output = RenderNode;

    fn render(
        &self,
        cx: &mut Context<'_>,
    ) -> impl zintl_ui::element::IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
