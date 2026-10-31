//! Value controls bound to Stores.

use std::rc::Rc;

use zintl_ui::element::IntoElement;
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_backend_core::ViewKind;
use zintl_ui_layout::{LayoutStyle, Size};

use crate::attach::attachments;
use crate::{Attachments, Color, Context, Element, Event, EventKind, NativeView, RenderNode};

type Change<T> = dyn for<'a> Fn(&mut Context<'a>, T);

/// Implements shared builder methods and `View` for controls.
macro_rules! control {
    ($($control:ident),* $(,)?) => {$(
        impl $control {
            pub fn id(mut self, id: impl Into<String>) -> Self {
                self.id = Some(id.into());
                self
            }

            pub fn minimum_size(mut self, size: Size) -> Self {
                self.layout.minimum_size = size;
                self
            }

            /// Stretches the control across its parent's width.
            pub fn fill_width(mut self) -> Self {
                self.layout.width = zintl_ui_layout::LayoutDimension::Percent(1.0);
                self
            }
        }

        impl View for $control {
            type Output = RenderNode;

            fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
                self.native_element(cx)
            }
        }
    )*};
}

macro_rules! enabled {
    ($($control:ident),* $(,)?) => {$(
        impl $control {
            pub fn enabled(mut self, enabled: bool) -> Self {
                self.enabled = enabled;
                self
            }
        }
    )*};
}

control!(
    Switch,
    Slider,
    PopUpButton,
    SegmentedControl,
    ComboBox,
    NumberField,
    SearchField,
    ColorWell,
);
enabled!(
    Switch,
    Slider,
    PopUpButton,
    SegmentedControl,
    ComboBox,
    NumberField,
    ColorWell,
);

/// Registers a handler that writes event values to `store` and then calls `action`.
struct Binding<T: 'static> {
    store: Option<Store<T>>,
    action: Option<Rc<Change<T>>>,
}

impl<T: Clone + 'static> Binding<T> {
    fn route(
        self,
        element: Element<RenderNode>,
        kind: EventKind,
        value: impl Fn(Event) -> Option<T> + 'static,
    ) -> Element<RenderNode> {
        if self.store.is_none() && self.action.is_none() {
            return element;
        }
        element.on_event(kind, move |cx, event| {
            let Some(value) = value(event) else {
                return;
            };
            if let Some(store) = self.store {
                let stored = value.clone();
                cx.update(store, |current| *current = stored);
            }
            if let Some(action) = &self.action {
                action(cx, value);
            }
        })
    }
}

/// A native on/off switch.
#[derive(Clone)]
pub struct Switch {
    on: bool,
    store: Option<Store<bool>>,
    action: Option<Rc<Change<bool>>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl Switch {
    pub fn new() -> Self {
        Self {
            on: false,
            store: None,
            action: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(38.0, 22.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn bind(mut self, store: Store<bool>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, bool) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl Default for Switch {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for Switch {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::Switch {
                on: self.store.map_or(self.on, |store| *cx.get(store)),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
        });
        Binding {
            store: self.store,
            action: self.action.clone(),
        }
        .route(element, EventKind::StateChanged, |event| match event {
            Event::StateChanged { on } => Some(on),
            _ => None,
        })
    }
}

/// A continuous native slider.
#[derive(Clone)]
pub struct Slider {
    value: f64,
    minimum: f64,
    maximum: f64,
    tick_marks: usize,
    snaps_to_ticks: bool,
    store: Option<Store<f64>>,
    action: Option<Rc<Change<f64>>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl Slider {
    pub fn new(minimum: f64, maximum: f64) -> Self {
        Self {
            value: minimum,
            minimum,
            maximum,
            tick_marks: 0,
            snaps_to_ticks: false,
            store: None,
            action: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(160.0, 24.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Shows `count` tick marks; `snap` restricts values to them.
    pub fn tick_marks(mut self, count: usize, snap: bool) -> Self {
        self.tick_marks = count;
        self.snaps_to_ticks = snap;
        self
    }

    pub fn bind(mut self, store: Store<f64>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, f64) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl NativeView for Slider {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::Slider {
                value: self.store.map_or(self.value, |store| *cx.get(store)),
                minimum: self.minimum,
                maximum: self.maximum,
                tick_marks: self.tick_marks,
                snaps_to_ticks: self.snaps_to_ticks,
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
        });
        Binding {
            store: self.store,
            action: self.action.clone(),
        }
        .route(element, EventKind::ValueChanged, |event| match event {
            Event::ValueChanged { value } => Some(value),
            _ => None,
        })
    }
}

impl Binding<usize> {
    /// Routes selection indexes for choice controls.
    fn route_selection(self, element: Element<RenderNode>) -> Element<RenderNode> {
        self.route(element, EventKind::SelectionChanged, |event| match event {
            Event::SelectionChanged { index } => Some(index),
            _ => None,
        })
    }
}

/// A native pop-up button that selects one item.
#[derive(Clone)]
pub struct PopUpButton {
    items: Vec<String>,
    selected: Option<usize>,
    store: Option<Store<usize>>,
    action: Option<Rc<Change<usize>>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl PopUpButton {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        let width = items
            .iter()
            .map(|item| item.chars().count())
            .max()
            .unwrap_or(0) as f32
            * 7.0
            + 44.0;
        Self {
            selected: (!items.is_empty()).then_some(0),
            items,
            store: None,
            action: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(width.max(100.0), 26.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    /// Reads and writes the selected item index.
    pub fn bind(mut self, store: Store<usize>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, usize) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl NativeView for PopUpButton {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::PopUpButton {
                items: self.items.clone(),
                selected: self.store.map(|store| *cx.get(store)).or(self.selected),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
        });
        Binding {
            store: self.store,
            action: self.action.clone(),
        }
        .route_selection(element)
    }
}

/// A native segmented control that selects one segment.
#[derive(Clone)]
pub struct SegmentedControl {
    segments: Vec<String>,
    selected: Option<usize>,
    store: Option<Store<usize>>,
    action: Option<Rc<Change<usize>>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl SegmentedControl {
    pub fn new(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let segments: Vec<String> = segments.into_iter().map(Into::into).collect();
        let width = segments
            .iter()
            .map(|segment| segment.chars().count() as f32 * 7.0 + 24.0)
            .sum::<f32>();
        Self {
            selected: (!segments.is_empty()).then_some(0),
            segments,
            store: None,
            action: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(width, 24.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    /// Reads and writes the selected segment index.
    pub fn bind(mut self, store: Store<usize>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, usize) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl NativeView for SegmentedControl {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::SegmentedControl {
                segments: self.segments.clone(),
                selected: self.store.map(|store| *cx.get(store)).or(self.selected),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
        });
        Binding {
            store: self.store,
            action: self.action.clone(),
        }
        .route_selection(element)
    }
}

/// An editable native combo box with suggested values.
#[derive(Clone)]
pub struct ComboBox {
    items: Vec<String>,
    value: String,
    placeholder: Option<String>,
    store: Option<Store<String>>,
    action: Option<Rc<Change<String>>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl ComboBox {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            value: String::new(),
            placeholder: None,
            store: None,
            action: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(160.0, 26.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn bind(mut self, store: Store<String>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, String) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl NativeView for ComboBox {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::ComboBox {
                items: self.items.clone(),
                value: self
                    .store
                    .map_or_else(|| self.value.clone(), |store| cx.get(store).clone()),
                placeholder: self.placeholder.clone(),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
        });
        Binding {
            store: self.store,
            action: self.action.clone(),
        }
        .route(element, EventKind::TextChanged, |event| match event {
            Event::TextChanged { value } => Some(value),
            _ => None,
        })
    }
}

/// A number-only text field with a stepper.
#[derive(Clone)]
pub struct NumberField {
    value: f64,
    minimum: f64,
    maximum: f64,
    step: f64,
    fraction_digits: usize,
    store: Option<Store<f64>>,
    action: Option<Rc<Change<f64>>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl NumberField {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            minimum: f64::MIN,
            maximum: f64::MAX,
            step: 1.0,
            fraction_digits: 0,
            store: None,
            action: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(100.0, 22.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Limits accepted values to `minimum..=maximum`.
    pub fn range(mut self, minimum: f64, maximum: f64) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    /// Sets the stepper increment.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Shows and accepts `digits` decimal places; zero accepts integers only.
    pub fn fraction_digits(mut self, digits: usize) -> Self {
        self.fraction_digits = digits;
        self
    }

    pub fn bind(mut self, store: Store<f64>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, f64) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl Default for NumberField {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for NumberField {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::NumberField {
                value: self.store.map_or(self.value, |store| *cx.get(store)),
                format: zintl_ui_backend_core::NumberFormat {
                    minimum: self.minimum,
                    maximum: self.maximum,
                    step: self.step,
                    fraction_digits: self.fraction_digits,
                },
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
        });
        let (minimum, maximum) = (self.minimum, self.maximum);
        Binding {
            store: self.store,
            action: self.action.clone(),
        }
        .route(element, EventKind::ValueChanged, move |event| match event {
            Event::ValueChanged { value } => Some(value.clamp(minimum, maximum)),
            _ => None,
        })
    }
}

/// A native search field that reports every edit.
#[derive(Clone)]
pub struct SearchField {
    value: String,
    placeholder: Option<String>,
    store: Option<Store<String>>,
    action: Option<Rc<Change<String>>>,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl SearchField {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: None,
            store: None,
            action: None,
            layout: LayoutStyle::leaf(Size::new(160.0, 22.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn bind(mut self, store: Store<String>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, String) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl Default for SearchField {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for SearchField {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::SearchField {
                value: self
                    .store
                    .map_or_else(|| self.value.clone(), |store| cx.get(store).clone()),
                placeholder: self.placeholder.clone(),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: true,
            title_bar_interactive: false,
        });
        Binding {
            store: self.store,
            action: self.action.clone(),
        }
        .route(element, EventKind::TextChanged, |event| match event {
            Event::TextChanged { value } => Some(value),
            _ => None,
        })
    }
}

/// A native color well that edits a color with the system color picker.
#[derive(Clone)]
pub struct ColorWell {
    color: Color,
    store: Option<Store<Color>>,
    action: Option<Rc<Change<Color>>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl ColorWell {
    pub fn new() -> Self {
        Self {
            color: Color::WHITE,
            store: None,
            action: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(44.0, 24.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn bind(mut self, store: Store<Color>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, Color) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl Default for ColorWell {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for ColorWell {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::ColorWell {
                color: self.store.map_or(self.color, |store| *cx.get(store)),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
        });
        Binding {
            store: self.store,
            action: self.action.clone(),
        }
        .route(element, EventKind::ColorChanged, |event| match event {
            Event::ColorChanged { color } => Some(color),
            _ => None,
        })
    }
}
