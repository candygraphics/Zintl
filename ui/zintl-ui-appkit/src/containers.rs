use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_layout::{Axis, LayoutDimension, LayoutStyle, Size};

use crate::{Context, Element, Event, EventKind, NativeView, RenderNode, ScrollAxes, ScrollStatus};

type SelectionAction = dyn for<'a> Fn(&mut Context<'a>, usize);

/// A native `NSScrollView` around its content.
#[derive(Clone)]
pub struct ScrollView {
    content: ElementFactory<RenderNode>,
    axes: ScrollAxes,
    store: Option<Store<ScrollStatus>>,
    layout: LayoutStyle,
    id: Option<String>,
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
            layout: LayoutStyle::fill(Size::new(80.0, 80.0)),
            id: None,
        }
    }

    pub fn axes(mut self, axes: ScrollAxes) -> Self {
        self.axes = axes;
        self
    }

    /// Saves the scroll position and restores it when the view is created again.
    pub fn bind(mut self, store: Store<ScrollStatus>) -> Self {
        self.store = Some(store);
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

    /// Uses a fixed height instead of filling the remaining space.
    pub fn height(mut self, height: f32) -> Self {
        self.layout.height = LayoutDimension::Points(height);
        self.layout.grow = 0.0;
        self
    }
}

impl NativeView for ScrollView {
    fn native_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::NSScrollView {
            axes: self.axes,
            status: self.store.map(|store| *cx.get(store)),
            layout: self.layout,
            id: self.id.clone(),
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

impl View for ScrollView {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}

/// One page of a [`TabView`].
#[derive(Clone)]
pub struct TabViewItem {
    title: String,
    content: ElementFactory<RenderNode>,
}

impl TabViewItem {
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

/// A native `NSTabView` that shows one page at a time.
#[derive(Clone)]
pub struct TabView {
    items: Vec<TabViewItem>,
    selected: usize,
    store: Option<Store<usize>>,
    action: Option<Rc<SelectionAction>>,
    layout: LayoutStyle,
    id: Option<String>,
}

impl TabView {
    /// Inset between the tab view's content area and each page.
    const PAGE_PADDING: f32 = 12.0;

    pub fn new(items: impl IntoIterator<Item = TabViewItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
            selected: 0,
            store: None,
            action: None,
            layout: LayoutStyle::fill(Size::new(200.0, 120.0)),
            id: None,
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Reads and writes the selected page index.
    pub fn bind(mut self, store: Store<usize>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, usize) + 'static) -> Self {
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
}

impl NativeView for TabView {
    fn native_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let mut page_layout = LayoutStyle::stack(Axis::Vertical, 8.0);
        page_layout.padding = Self::PAGE_PADDING;
        let pages = self.items.iter().map(|item| {
            Element::node(RenderNode::NSView {
                layout: page_layout,
                id: None,
            })
            .with_children([item.content.clone().into_element()])
        });
        let element = Element::node(RenderNode::NSTabView {
            titles: self.items.iter().map(|item| item.title.clone()).collect(),
            selected: self.store.map_or(self.selected, |store| *cx.get(store)),
            layout: self.layout,
            id: self.id.clone(),
        })
        .with_children(pages);
        if self.store.is_none() && self.action.is_none() {
            return element;
        }
        let store = self.store;
        let action = self.action.clone();
        element.on_event(EventKind::SelectionChanged, move |cx, event| {
            if let Event::SelectionChanged { index } = event {
                if let Some(store) = store {
                    cx.update(store, |selected| *selected = index);
                }
                if let Some(action) = &action {
                    action(cx, index);
                }
            }
        })
    }
}

impl View for TabView {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
