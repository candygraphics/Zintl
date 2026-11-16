//! Value controls bound to Stores.

use std::ops::RangeInclusive;
use std::rc::Rc;

use zintl_ui::element::IntoElement;
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_backend_core::{CalendarDate, DatePickerStyle, ViewKind};
use zintl_ui_layout::{LayoutStyle, Size};

use crate::modifiers;
use crate::{
    Attach, Change, Color, Context, Control, Element, Event, EventKind, NativeView, RenderNode,
    ViewModifiers,
};

native_view!(
    Switch,
    Slider,
    PopUpButton,
    SegmentedControl,
    ComboBox,
    NumberField,
    SearchField,
    ColorWell,
    DatePicker,
);

/// Writes a control's new value to its bound Store, then reports it to `on_change`.
pub(crate) struct ChangeRoute<T: 'static> {
    store: Option<Store<T>>,
    action: Option<Rc<Change<T>>>,
}

impl<T: 'static> Clone for ChangeRoute<T> {
    fn clone(&self) -> Self {
        Self {
            store: self.store,
            action: self.action.clone(),
        }
    }
}

impl<T: 'static> Default for ChangeRoute<T> {
    fn default() -> Self {
        Self {
            store: None,
            action: None,
        }
    }
}

impl<T: Copy + 'static> ChangeRoute<T> {
    /// Wraps an `on_change` action that takes its value by copy.
    pub(crate) fn copied(action: impl for<'a> Fn(&mut Context<'a>, T) + 'static) -> Rc<Change<T>> {
        Rc::new(move |cx: &mut Context<'_>, value: &T| action(cx, *value))
    }
}

impl ChangeRoute<String> {
    /// Wraps an `on_change` action that takes text as `&str`.
    pub(crate) fn text(
        action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static,
    ) -> Rc<Change<String>> {
        Rc::new(move |cx: &mut Context<'_>, value: &String| action(cx, value))
    }
}

impl<T: 'static> ChangeRoute<T> {
    pub(crate) fn new(store: Option<Store<T>>, action: Option<Rc<Change<T>>>) -> Self {
        Self { store, action }
    }

    /// The bound Store, whose value the control shows.
    pub(crate) fn store(&self) -> Option<Store<T>> {
        self.store
    }

    /// Writes `value` to the Store, then calls the action.
    pub(crate) fn send(&self, cx: &mut Context<'_>, value: T)
    where
        T: Clone,
    {
        let Some(action) = &self.action else {
            if let Some(store) = self.store {
                cx.update(store, |current| *current = value);
            }
            return;
        };
        if let Some(store) = self.store {
            let stored = value.clone();
            cx.update(store, |current| *current = stored);
        }
        action(cx, &value);
    }

    /// Sends the value `value` reads from each `kind` event.
    pub(crate) fn route(
        self,
        element: Element<RenderNode>,
        kind: EventKind,
        value: impl Fn(Event) -> Option<T> + 'static,
    ) -> Element<RenderNode>
    where
        T: Clone,
    {
        if self.store.is_none() && self.action.is_none() {
            return element;
        }
        element.on_event(kind, move |cx, event| {
            if let Some(value) = value(event) {
                self.send(cx, value);
            }
        })
    }
}

/// A native on/off switch.
#[derive(Clone)]
pub(crate) struct Switch {
    on: bool,
    store: Option<Store<bool>>,
    action: Option<Rc<Change<bool>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl Switch {
    pub fn new() -> Self {
        Self {
            on: false,
            store: None,
            action: None,
            layout: LayoutStyle::leaf(Size::new(38.0, 22.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn value(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Reads the shown state from the route's Store and reports changes to it.
    pub fn route(mut self, route: ChangeRoute<bool>) -> Self {
        self.store = route.store;
        self.action = route.action;
        self
    }
}

impl Default for Switch {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for Switch {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::Switch {
                on: self.store.map_or(self.on, |store| *cx.get(store)),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        ChangeRoute::new(self.store, self.action.clone()).route(
            element,
            EventKind::StateChanged,
            |event| match event {
                Event::StateChanged { on } => Some(on),
                _ => None,
            },
        )
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
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl Slider {
    pub fn new(range: RangeInclusive<f64>) -> Self {
        let (minimum, maximum) = range.into_inner();
        Self {
            value: minimum,
            minimum,
            maximum,
            tick_marks: 0,
            snaps_to_ticks: false,
            store: None,
            action: None,
            layout: LayoutStyle::leaf(Size::new(160.0, 24.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Shows `count` tick marks below the track.
    pub fn ticks(mut self, count: usize) -> Self {
        self.tick_marks = count;
        self.snaps_to_ticks = false;
        self
    }

    /// Shows `count` tick marks and only allows their values.
    pub fn snapping_ticks(mut self, count: usize) -> Self {
        self.tick_marks = count;
        self.snaps_to_ticks = true;
        self
    }

    pub fn bind(mut self, store: Store<f64>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, f64) + 'static) -> Self {
        self.action = Some(ChangeRoute::copied(action));
        self
    }
}

impl NativeView for Slider {
    modifiers!();

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
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        ChangeRoute::new(self.store, self.action.clone()).route(
            element,
            EventKind::ValueChanged,
            |event| match event {
                Event::ValueChanged { value } => Some(value),
                _ => None,
            },
        )
    }
}

impl ChangeRoute<usize> {
    /// Routes selection indexes for choice controls.
    pub(crate) fn route_selection(self, element: Element<RenderNode>) -> Element<RenderNode> {
        self.route(element, EventKind::SelectionChanged, |event| match event {
            Event::SelectionChanged { index } => Some(index),
            _ => None,
        })
    }
}

/// A native pop-up button that selects one item.
#[derive(Clone)]
pub(crate) struct PopUpButton {
    items: Vec<String>,
    selected: Option<usize>,
    store: Option<Store<usize>>,
    action: Option<Rc<Change<usize>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
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
            layout: LayoutStyle::leaf(Size::new(width.max(100.0), 26.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn selection(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    /// Reads the selected item index from the route's Store and reports changes to it.
    pub fn route(mut self, route: ChangeRoute<usize>) -> Self {
        self.store = route.store;
        self.action = route.action;
        self
    }
}

impl NativeView for PopUpButton {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::PopUpButton {
                items: self.items.clone(),
                selected: self.store.map(|store| *cx.get(store)).or(self.selected),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        ChangeRoute::new(self.store, self.action.clone()).route_selection(element)
    }
}

/// A native segmented control that selects one segment.
#[derive(Clone)]
pub(crate) struct SegmentedControl {
    segments: Vec<String>,
    selected: Option<usize>,
    store: Option<Store<usize>>,
    action: Option<Rc<Change<usize>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
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
            layout: LayoutStyle::leaf(Size::new(width, 24.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn selection(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    /// Reads the selected segment index from the route's Store and reports changes to it.
    pub fn route(mut self, route: ChangeRoute<usize>) -> Self {
        self.store = route.store;
        self.action = route.action;
        self
    }
}

impl NativeView for SegmentedControl {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::SegmentedControl {
                segments: self.segments.clone(),
                selected: self.store.map(|store| *cx.get(store)).or(self.selected),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        ChangeRoute::new(self.store, self.action.clone()).route_selection(element)
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
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl ComboBox {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            value: String::new(),
            placeholder: None,
            store: None,
            action: None,
            layout: LayoutStyle::leaf(Size::new(160.0, 26.0)),
            modifiers: ViewModifiers::default(),
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

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static) -> Self {
        self.action = Some(ChangeRoute::text(action));
        self
    }
}

impl NativeView for ComboBox {
    modifiers!();

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
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        ChangeRoute::new(self.store, self.action.clone()).route(
            element,
            EventKind::TextChanged,
            |event| match event {
                Event::TextChanged { value } => Some(value),
                _ => None,
            },
        )
    }
}

type FormatValue = dyn Fn(f64) -> String;
type ParseValue = dyn Fn(&str) -> Option<f64>;

/// A number-only text field with a stepper.
///
/// By default the platform formats and parses the number. With
/// [`NumberField::formatter`] or [`NumberField::parser`] the field shows the
/// formatted text and parses it while the user types and once editing ends;
/// text that does not parse restores the current value when editing ends.
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
    layout: LayoutStyle,
    modifiers: ViewModifiers,
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
            layout: LayoutStyle::leaf(Size::new(100.0, 22.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Limits accepted values to `range`.
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        (self.minimum, self.maximum) = range.into_inner();
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
        self.action = Some(ChangeRoute::copied(action));
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
    modifiers!();

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
                placeholder: None,
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        let binding = || ChangeRoute::new(self.store, self.action.clone());
        let (minimum, maximum) = (self.minimum, self.maximum);
        let element = binding().route(element, EventKind::ValueChanged, move |event| match event {
            Event::ValueChanged { value } => Some(value.clamp(minimum, maximum)),
            _ => None,
        });
        if !self.formats_text() {
            return element;
        }
        // Typing reports each number, so actions see the value before editing ends.
        let field = self.clone();
        let element = binding().route(element, EventKind::TextChanged, move |event| match event {
            Event::TextChanged { value: text } => {
                field.parse(&text).filter(|parsed| *parsed != value)
            }
            _ => None,
        });
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

/// A value an [`IntegerField`] edits: `i64`, or `Option<i64>` where an empty field is `None`.
pub trait IntegerValue: Copy + PartialEq + 'static {
    /// Returns the integer, or `None` for an empty field.
    fn integer(self) -> Option<i64>;

    /// Returns the value for `integer`, or `None` when this type cannot be empty.
    fn from_integer(integer: Option<i64>) -> Option<Self>;
}

impl IntegerValue for i64 {
    fn integer(self) -> Option<i64> {
        Some(self)
    }

    fn from_integer(integer: Option<i64>) -> Option<Self> {
        integer
    }
}

impl IntegerValue for Option<i64> {
    fn integer(self) -> Option<i64> {
        self
    }

    fn from_integer(integer: Option<i64>) -> Option<Self> {
        Some(integer)
    }
}

/// An integer field with a stepper whose values never pass through `f64`.
///
/// The field formats and parses `i64` itself, so values above 2^53 stay
/// exact. Values are reported while the user types, whenever the text is an
/// integer; once editing ends the text is formatted again, and text that is
/// not an integer restores the last value, so `12.5` ends as `12`. Full-width
/// digits from input methods are accepted.
///
/// `IntegerField<Option<i64>>` shows `None` as an empty field.
#[derive(Clone)]
pub struct IntegerField<T: IntegerValue = i64> {
    value: T,
    minimum: i64,
    maximum: i64,
    step: i64,
    grouping: bool,
    suffix: String,
    placeholder: Option<String>,
    store: Option<Store<T>>,
    action: Option<Rc<Change<T>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl IntegerField<i64> {
    pub fn new() -> Self {
        Self::with_value(0)
    }
}

impl IntegerField<Option<i64>> {
    /// Creates a field whose empty text is `None`.
    pub fn optional() -> Self {
        Self::with_value(None)
    }

    /// Shows `placeholder` while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }
}

impl<T: IntegerValue> IntegerField<T> {
    fn with_value(value: T) -> Self {
        Self {
            value,
            minimum: i64::MIN,
            maximum: i64::MAX,
            step: 1,
            grouping: false,
            suffix: String::new(),
            placeholder: None,
            store: None,
            action: None,
            layout: LayoutStyle::leaf(Size::new(100.0, 22.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn value(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    /// Limits accepted values to `range`.
    pub fn range(mut self, range: RangeInclusive<i64>) -> Self {
        (self.minimum, self.maximum) = range.into_inner();
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

    pub fn bind(mut self, store: Store<T>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, T) + 'static) -> Self {
        self.action = Some(ChangeRoute::copied(action));
        self
    }

    /// Formats `value` with the separators and suffix this field shows; `None` is empty.
    pub fn format(&self, value: T) -> String {
        let Some(value) = value.integer() else {
            return String::new();
        };
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
    /// Empty text is `None` for `Option<i64>` fields and rejected otherwise.
    /// Full-width digits and minus signs from input methods are accepted.
    pub fn parse(&self, text: &str) -> Option<T> {
        let text = text.trim();
        if text.is_empty() {
            return T::from_integer(None);
        }
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
        T::from_integer(Some(i64::try_from(clamped).ok()?))
    }

    /// Returns the value one stepper click moves `value` to; an empty field starts from zero.
    pub fn stepped(&self, value: T, up: bool) -> Option<T> {
        let current = value
            .integer()
            .unwrap_or(0)
            .clamp(self.minimum, self.maximum);
        let next = if up {
            current.saturating_add(self.step)
        } else {
            current.saturating_sub(self.step)
        };
        T::from_integer(Some(next.clamp(self.minimum, self.maximum))).filter(|next| *next != value)
    }
}

impl Default for IntegerField<i64> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IntegerValue> NativeView for IntegerField<T> {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let value = self.store.map_or(self.value, |store| *cx.get(store));
        // An empty field steps from zero, so the stepper starts there.
        let stepper = value
            .integer()
            .unwrap_or(0)
            .clamp(self.minimum, self.maximum);
        // The field shows text formatted here; the stepper only needs the direction of a click.
        let element = Element::node(RenderNode::View {
            kind: ViewKind::NumberField {
                value: stepper as f64,
                format: zintl_ui_backend_core::NumberFormat {
                    minimum: self.minimum as f64,
                    maximum: self.maximum as f64,
                    step: self.step as f64,
                    fraction_digits: 0,
                },
                text: Some(self.format(value)),
                placeholder: self.placeholder.clone(),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        let binding = || ChangeRoute::new(self.store, self.action.clone());
        let field = self.clone();
        let element = binding().route(element, EventKind::ValueChanged, move |event| match event {
            // The stepper reports a rounded value, so only its direction is used.
            Event::ValueChanged { value: shown } => field.stepped(value, shown > stepper as f64),
            _ => None,
        });
        // Typing reports each integer, so actions see the value before editing ends.
        let field = self.clone();
        let element = binding().route(element, EventKind::TextChanged, move |event| match event {
            Event::TextChanged { value: text } => {
                field.parse(&text).filter(|parsed| *parsed != value)
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

impl<T: IntegerValue> View for IntegerField<T> {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}

/// Picks a calendar date, in a text field with a stepper or a month calendar.
///
/// The date has no time or time zone, so it is the same day everywhere.
/// The picker formats it for the system locale.
#[derive(Clone)]
pub struct DatePicker {
    date: CalendarDate,
    style: DatePickerStyle,
    store: Option<Store<CalendarDate>>,
    action: Option<Rc<Change<CalendarDate>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl DatePicker {
    /// The date an unbound picker shows until [`DatePicker::value`] sets one.
    const DEFAULT_DATE: CalendarDate = CalendarDate {
        year: 2000,
        month: 1,
        day: 1,
    };

    pub fn new() -> Self {
        Self {
            date: Self::DEFAULT_DATE,
            style: DatePickerStyle::TextField,
            store: None,
            action: None,
            layout: LayoutStyle::leaf(Size::new(110.0, 22.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn value(mut self, date: CalendarDate) -> Self {
        self.date = date;
        self
    }

    /// Shows the date in a text field with a stepper, or as a month calendar.
    pub fn style(mut self, style: DatePickerStyle) -> Self {
        self.style = style;
        self
    }

    pub fn bind(mut self, store: Store<CalendarDate>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, CalendarDate) + 'static,
    ) -> Self {
        self.action = Some(ChangeRoute::copied(action));
        self
    }
}

impl Default for DatePicker {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for DatePicker {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::DatePicker {
                date: self.store.map_or(self.date, |store| *cx.get(store)),
                style: self.style,
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        let binding = || ChangeRoute::new(self.store, self.action.clone());
        let element = binding().route(element, EventKind::DateChanged, |event| match event {
            Event::DateChanged { date } => Some(date),
            _ => None,
        });
        // Backends without a native date picker show an ISO date in a text field.
        binding().route(element, EventKind::TextCommitted, |event| match event {
            Event::TextCommitted { value } => CalendarDate::parse_iso(&value),
            _ => None,
        })
    }
}

impl DatePicker {
    /// Creates a picker for a date that may be missing, bound to a `Store<Option<CalendarDate>>`.
    pub fn optional() -> OptionalDatePicker {
        OptionalDatePicker::new()
    }
}

/// A date picker that can show "no date", for values such as an optional due date.
///
/// Without a date it shows a button with the placeholder; clicking it picks
/// today's date. With a date it shows the picker and a clear button beside it.
/// The picker and the placeholder button carry the ID; the clear button's ID
/// adds `-clear`, such as `due-clear`.
#[derive(Clone)]
pub struct OptionalDatePicker {
    date: Option<CalendarDate>,
    placeholder: String,
    style: DatePickerStyle,
    store: Option<Store<Option<CalendarDate>>>,
    action: Option<Rc<Change<Option<CalendarDate>>>>,
    modifiers: ViewModifiers,
}

impl OptionalDatePicker {
    fn new() -> Self {
        Self {
            date: None,
            placeholder: "No Date".into(),
            style: DatePickerStyle::TextField,
            store: None,
            action: None,
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn value(mut self, date: Option<CalendarDate>) -> Self {
        self.date = date;
        self
    }

    /// Sets the button title shown without a date.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Shows the date in a text field with a stepper, or as a month calendar.
    pub fn style(mut self, style: DatePickerStyle) -> Self {
        self.style = style;
        self
    }

    pub fn bind(mut self, store: Store<Option<CalendarDate>>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, Option<CalendarDate>) + 'static,
    ) -> Self {
        self.action = Some(ChangeRoute::copied(action));
        self
    }

    /// Returns an action that writes `date` to the Store and reports it.
    fn change(&self, date: Option<CalendarDate>) -> impl for<'a> Fn(&mut Context<'a>) + 'static {
        let route = ChangeRoute::new(self.store, self.action.clone());
        move |cx| route.send(cx, date)
    }
}

impl NativeView for OptionalDatePicker {
    modifiers!();

    // The picker and the buttons carry the ID, not the row.
    fn places_id(&self) -> bool {
        true
    }

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let date = self.store.map_or(self.date, |store| *cx.get(store));
        let (id, enabled) = (self.modifiers.id(), self.modifiers.enabled());
        let mut row = LayoutStyle::stack(zintl_ui_layout::Axis::Horizontal, 4.0);
        row.cross_axis_alignment = zintl_ui_layout::CrossAxisAlignment::Center;
        let children = match date {
            None => {
                let pick = self.change(Some(CalendarDate::today()));
                let mut button = crate::Button::new(self.placeholder.clone())
                    .enabled(enabled)
                    .on_click(pick);
                if let Some(id) = id {
                    button = button.id(id);
                }
                zintl_ui::list![button]
            }
            Some(date) => {
                let route = ChangeRoute::new(self.store, self.action.clone());
                let mut picker = DatePicker::new()
                    .value(date)
                    .style(self.style)
                    .enabled(enabled)
                    .on_change(move |cx, date| route.send(cx, Some(date)));
                let mut clear = crate::Button::new("")
                    .system_image("xmark.circle.fill")
                    .style(crate::ButtonStyle::Borderless)
                    .enabled(enabled)
                    .on_click(self.change(None))
                    .accessibility_label("Clear Date")
                    .help("Clear Date");
                if let Some(id) = id {
                    picker = picker.id(id);
                    clear = clear.id(format!("{id}-clear"));
                }
                zintl_ui::list![picker, clear]
            }
        };
        crate::Container::new(row, children).native_element(cx)
    }
}

native_view!(OptionalDatePicker);

/// A native search field that reports every edit.
#[derive(Clone)]
pub struct SearchField {
    value: String,
    placeholder: Option<String>,
    store: Option<Store<String>>,
    action: Option<Rc<Change<String>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl SearchField {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: None,
            store: None,
            action: None,
            layout: LayoutStyle::leaf(Size::new(160.0, 22.0)),
            modifiers: ViewModifiers::default(),
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

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static) -> Self {
        self.action = Some(ChangeRoute::text(action));
        self
    }
}

impl Default for SearchField {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for SearchField {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::SearchField {
                value: self
                    .store
                    .map_or_else(|| self.value.clone(), |store| cx.get(store).clone()),
                placeholder: self.placeholder.clone(),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        ChangeRoute::new(self.store, self.action.clone()).route(
            element,
            EventKind::TextChanged,
            |event| match event {
                Event::TextChanged { value } => Some(value),
                _ => None,
            },
        )
    }
}

/// A native color well that edits a color with the system color picker.
#[derive(Clone)]
pub struct ColorWell {
    color: Color,
    store: Option<Store<Color>>,
    action: Option<Rc<Change<Color>>>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl ColorWell {
    pub fn new() -> Self {
        Self {
            color: Color::WHITE,
            store: None,
            action: None,
            layout: LayoutStyle::leaf(Size::new(44.0, 24.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn value(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn bind(mut self, store: Store<Color>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, Color) + 'static) -> Self {
        self.action = Some(ChangeRoute::copied(action));
        self
    }
}

impl Default for ColorWell {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for ColorWell {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::ColorWell {
                color: self.store.map_or(self.color, |store| *cx.get(store)),
            },
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        ChangeRoute::new(self.store, self.action.clone()).route(
            element,
            EventKind::ColorChanged,
            |event| match event {
                Event::ColorChanged { color } => Some(color),
                _ => None,
            },
        )
    }
}
