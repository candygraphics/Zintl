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
                self.layout.minimum_size = Some(size);
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
    IntegerField,
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
    IntegerField,
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
            help: None,
            accessibility: Default::default(),
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
            help: None,
            accessibility: Default::default(),
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
            help: None,
            accessibility: Default::default(),
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
            help: None,
            accessibility: Default::default(),
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
            help: None,
            accessibility: Default::default(),
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

type FormatValue = dyn Fn(f64) -> String;
type ParseValue = dyn Fn(&str) -> Option<f64>;

/// A number-only text field with a stepper.
///
/// By default the platform formats and parses the number. With
/// [`NumberField::formatter`] or [`NumberField::parser`] the field shows the
/// formatted text and parses it once editing ends; text that does not parse
/// restores the current value.
#[derive(Clone)]
pub struct NumberField {
    value: f64,
    minimum: f64,
    maximum: f64,
    step: f64,
    fraction_digits: usize,
    formatter: Option<Rc<FormatValue>>,
    parser: Option<Rc<ParseValue>>,
    suffix: String,
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
            formatter: None,
            parser: None,
            suffix: String::new(),
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

    /// Shows values as `format` returns them, such as `12k345.67`.
    pub fn formatter(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.formatter = Some(Rc::new(format));
        self
    }

    /// Reads typed text; `None` rejects it and restores the current value.
    ///
    /// The default parser accepts plain decimal numbers.
    pub fn parser(mut self, parse: impl Fn(&str) -> Option<f64> + 'static) -> Self {
        self.parser = Some(Rc::new(parse));
        self
    }

    /// Appends `suffix`, such as `" m"`, to the shown text; typed text may omit it.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
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

    /// Whether this field formats its text itself instead of the platform.
    fn formats_text(&self) -> bool {
        self.formatter.is_some() || self.parser.is_some() || !self.suffix.is_empty()
    }

    fn format(&self, value: f64) -> String {
        let text = match &self.formatter {
            Some(format) => format(value),
            None => format!("{value:.*}", self.fraction_digits),
        };
        format!("{text}{}", self.suffix)
    }

    /// Parses `text` without the suffix and limits it to the range.
    fn parse(&self, text: &str) -> Option<f64> {
        let text = text.trim();
        let text = text.strip_suffix(self.suffix.trim()).unwrap_or(text).trim();
        let value = match &self.parser {
            Some(parse) => parse(text),
            None => text.parse().ok(),
        }?;
        value
            .is_finite()
            .then(|| value.clamp(self.minimum, self.maximum))
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
        let value = self.store.map_or(self.value, |store| *cx.get(store));
        let element = Element::node(RenderNode::View {
            kind: ViewKind::NumberField {
                value,
                format: zintl_ui_backend_core::NumberFormat {
                    minimum: self.minimum,
                    maximum: self.maximum,
                    step: self.step,
                    fraction_digits: self.fraction_digits,
                },
                text: self.formats_text().then(|| self.format(value)),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        let binding = || Binding {
            store: self.store,
            action: self.action.clone(),
        };
        let (minimum, maximum) = (self.minimum, self.maximum);
        let element = binding().route(element, EventKind::ValueChanged, move |event| match event {
            Event::ValueChanged { value } => Some(value.clamp(minimum, maximum)),
            _ => None,
        });
        if !self.formats_text() {
            return element;
        }
        let field = self.clone();
        binding().route(
            element,
            EventKind::TextCommitted,
            move |event| match event {
                // Unchanged values are not reported again.
                Event::TextCommitted { value: text } => {
                    field.parse(&text).filter(|parsed| *parsed != value)
                }
                _ => None,
            },
        )
    }
}

/// An integer field with a stepper whose values never pass through `f64`.
///
/// The field formats and parses `i64` itself, so values above 2^53 stay
/// exact. Text that is not an integer, such as `12.5`, restores the current
/// value; full-width digits from input methods are accepted.
#[derive(Clone)]
pub struct IntegerField {
    value: i64,
    minimum: i64,
    maximum: i64,
    step: i64,
    grouping: bool,
    suffix: String,
    store: Option<Store<i64>>,
    action: Option<Rc<Change<i64>>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl IntegerField {
    pub fn new() -> Self {
        Self {
            value: 0,
            minimum: i64::MIN,
            maximum: i64::MAX,
            step: 1,
            grouping: false,
            suffix: String::new(),
            store: None,
            action: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(100.0, 22.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn value(mut self, value: i64) -> Self {
        self.value = value;
        self
    }

    /// Limits accepted values to `minimum..=maximum`.
    pub fn range(mut self, minimum: i64, maximum: i64) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    /// Sets the stepper increment.
    pub fn step(mut self, step: i64) -> Self {
        self.step = step;
        self
    }

    /// Shows thousands separators, such as `1,234`; typed text may use them either way.
    pub fn grouping(mut self, grouping: bool) -> Self {
        self.grouping = grouping;
        self
    }

    /// Appends `suffix`, such as `" pcs"`, to the shown text; typed text may omit it.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn bind(mut self, store: Store<i64>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, i64) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Formats `value` with the separators and suffix this field shows.
    pub fn format(&self, value: i64) -> String {
        let digits = value.unsigned_abs().to_string();
        let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
        for (index, digit) in digits.chars().enumerate() {
            if self.grouping && index > 0 && (digits.len() - index).is_multiple_of(3) {
                grouped.push(',');
            }
            grouped.push(digit);
        }
        let sign = if value < 0 { "-" } else { "" };
        format!("{sign}{grouped}{}", self.suffix)
    }

    /// Parses an integer, ignoring separators and the suffix, and limits it to the range.
    ///
    /// Full-width digits and minus signs from input methods are accepted.
    pub fn parse(&self, text: &str) -> Option<i64> {
        let text = text.trim();
        let text = text.strip_suffix(self.suffix.trim()).unwrap_or(text).trim();
        let normalized =
            text.chars()
                .filter(|character| !matches!(character, ',' | '，' | ' ' | '_'))
                .map(|character| match character {
                    '０'..='９' => char::from_u32(character as u32 - '０' as u32 + '0' as u32)
                        .unwrap_or(character),
                    '－' | '−' => '-',
                    _ => character,
                })
                .collect::<String>();
        let value: i128 = normalized.parse().ok()?;
        let clamped = value.clamp(i128::from(self.minimum), i128::from(self.maximum));
        i64::try_from(clamped).ok()
    }
}

impl Default for IntegerField {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for IntegerField {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let value = self.store.map_or(self.value, |store| *cx.get(store));
        // The field shows text formatted here; the stepper only needs the direction of a click.
        let element = Element::node(RenderNode::View {
            kind: ViewKind::NumberField {
                value: value as f64,
                format: zintl_ui_backend_core::NumberFormat {
                    minimum: self.minimum as f64,
                    maximum: self.maximum as f64,
                    step: self.step as f64,
                    fraction_digits: 0,
                },
                text: Some(self.format(value)),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        let binding = || Binding {
            store: self.store,
            action: self.action.clone(),
        };
        let (minimum, maximum, step) = (self.minimum, self.maximum, self.step);
        let element = binding().route(element, EventKind::ValueChanged, move |event| match event {
            // The stepper reports a rounded value, so only its direction from `value` is used.
            Event::ValueChanged { value: stepped } => {
                let next = if stepped > value as f64 {
                    value.saturating_add(step)
                } else {
                    value.saturating_sub(step)
                };
                Some(next.clamp(minimum, maximum)).filter(|next| *next != value)
            }
            _ => None,
        });
        let field = self.clone();
        binding().route(
            element,
            EventKind::TextCommitted,
            move |event| match event {
                Event::TextCommitted { value: text } => {
                    field.parse(&text).filter(|parsed| *parsed != value)
                }
                _ => None,
            },
        )
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
            help: None,
            accessibility: Default::default(),
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
            help: None,
            accessibility: Default::default(),
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
