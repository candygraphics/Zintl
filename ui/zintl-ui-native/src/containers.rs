use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::store::{Store, Trigger};
use zintl_ui_backend_core::{ScrollAlignment, ScrollTarget, ViewKind};
use zintl_ui_layout::{Axis, LayoutStyle, Size};

use crate::modifiers;
use crate::{
    Change, ChangeRoute, Context, Element, Event, EventKind, NativeView, RenderNode, ScrollAxes,
    ScrollStatus, ViewModifiers,
};

/// A native scroll view around its content.
#[derive(Clone)]
pub struct ScrollView {
    content: ElementFactory<RenderNode>,
    axes: ScrollAxes,
    store: Option<Store<ScrollStatus>>,
    target: Option<(Trigger<String>, ScrollAlignment)>,
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
    pub fn scroll_to(mut self, trigger: Trigger<String>, alignment: ScrollAlignment) -> Self {
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
                        id: cx.get(trigger.store()).clone()?,
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

/// One page of a [`TabView`].
#[derive(Clone)]
pub struct Tab {
    title: String,
    content: ElementFactory<RenderNode>,
}

impl Tab {
    pub fn new<V>(title: impl Into<String>, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            title: title.into(),
            content: ElementFactory::new(content),
        }
    }
}

/// A native tab view that shows one page at a time.
#[derive(Clone)]
pub struct TabView {
    items: Vec<Tab>,
    selected: usize,
    store: Option<Store<usize>>,
    action: Option<Rc<Change<usize>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl TabView {
    /// Inset between the tab view's content area and each page.
    const PAGE_PADDING: f32 = 12.0;

    pub fn new(items: impl IntoIterator<Item = Tab>) -> Self {
        Self {
            items: items.into_iter().collect(),
            selected: 0,
            store: None,
            action: None,
            layout: LayoutStyle::fill(Size::new(200.0, 120.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    /// Shows the page at `index`.
    pub fn selection(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Reads and writes the selected page index.
    pub fn bind(mut self, store: Store<usize>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, usize) + 'static) -> Self {
        self.action = Some(ChangeRoute::copied(action));
        self
    }
}

impl NativeView for TabView {
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
                selected: self.store.map_or(self.selected, |store| *cx.get(store)),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
        .with_children(pages);
        ChangeRoute::new(self.store, self.action.clone()).route_selection(element)
    }
}

native_view!(TabView);
