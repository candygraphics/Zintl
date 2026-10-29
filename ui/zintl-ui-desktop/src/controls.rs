//! Desktop controls that wrap native views.

use std::ops::RangeInclusive;

use zintl_ui::element::{Element, IntoElement};
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_layout::{Axis, LayoutStyle, Size};
use zintl_ui_native::NativeView;

use crate::{Attachments, Color, Context, ListFactory, RenderNode, ScrollAxes, ScrollStatus};

type Change<T> = dyn for<'a> Fn(&mut Context<'a>, T);

native_view!(
    Toggle,
    Picker,
    ComboBox,
    Slider,
    NumberField,
    SearchField,
    ColorWell,
    ScrollView,
    TabView,
);

/// How a [`Toggle`] is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToggleStyle {
    /// A label followed by a switch.
    #[default]
    Switch,
    /// A checkbox with its label beside the box.
    Checkbox,
    /// A push button that stays pressed while on.
    Button,
}

/// A two-state control bound to a `Store<bool>`.
#[derive(Clone)]
pub struct Toggle {
    label: String,
    style: ToggleStyle,
    store: Option<Store<bool>>,
    on: bool,
    enabled: bool,
    id: Option<String>,
    action: Option<std::rc::Rc<Change<bool>>>,
    attachments: Attachments,
}

impl Toggle {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            style: ToggleStyle::Switch,
            store: None,
            on: false,
            enabled: true,
            id: None,
            action: None,
            attachments: Attachments::default(),
        }
    }

    pub fn style(mut self, style: ToggleStyle) -> Self {
        self.style = style;
        self
    }

    pub fn bind(mut self, store: Store<bool>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, bool) + 'static) -> Self {
        self.action = Some(std::rc::Rc::new(action));
        self
    }

    fn button(&self, button: zintl_ui_native::Button) -> zintl_ui_native::Button {
        let mut button = button.on(self.on).enabled(self.enabled);
        if let Some(store) = self.store {
            button = button.bind_state(store);
        }
        if let Some(action) = self.action.clone() {
            button = button.on_state_change(move |cx, on| action(cx, on));
        }
        match &self.id {
            Some(id) => button.id(id.clone()),
            None => button,
        }
    }
}

impl NativeView for Toggle {
    fn attachments(&self) -> &Attachments {
        &self.attachments
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        &mut self.attachments
    }

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        match self.style {
            ToggleStyle::Checkbox => self
                .button(zintl_ui_native::Button::checkbox(self.label.clone()))
                .native_element(cx),
            ToggleStyle::Button => {
                let width = self.label.chars().count() as f32 * 7.0 + 32.0;
                self.button(
                    zintl_ui_native::Button::new(self.label.clone())
                        .button_type(zintl_ui_native::ButtonType::PushOnPushOff)
                        .minimum_size(Size::new(width.max(80.0), 32.0)),
                )
                .native_element(cx)
            }
            ToggleStyle::Switch => {
                let mut switch = zintl_ui_native::Switch::new()
                    .on(self.on)
                    .enabled(self.enabled);
                if let Some(store) = self.store {
                    switch = switch.bind(store);
                }
                if let Some(action) = self.action.clone() {
                    switch = switch.on_change(move |cx, on| action(cx, on));
                }
                if let Some(id) = &self.id {
                    switch = switch.id(id.clone());
                }
                if self.label.is_empty() {
                    return switch.native_element(cx);
                }
                // The label leads and the switch trails, as in System Settings.
                let mut row = LayoutStyle::stack(Axis::Horizontal, 12.0);
                row.main_axis_distribution = zintl_ui_layout::MainAxisDistribution::SpaceBetween;
                zintl_ui_native::View::new(
                    row,
                    zintl_ui_native::list![
                        zintl_ui_native::TextField::label_with_string(self.label.clone()),
                        switch,
                    ],
                )
                .native_element(cx)
            }
        }
    }
}

/// How a [`Picker`] presents its options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PickerStyle {
    /// A pop-up button.
    #[default]
    Menu,
    /// A segmented control.
    Segmented,
    /// A vertical group of radio buttons.
    Radio,
}

/// Selects one option by index.
#[derive(Clone)]
pub struct Picker {
    options: Vec<String>,
    style: PickerStyle,
    store: Option<Store<usize>>,
    selected: usize,
    enabled: bool,
    id: Option<String>,
    action: Option<std::rc::Rc<Change<usize>>>,
    attachments: Attachments,
}

impl Picker {
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            style: PickerStyle::Menu,
            store: None,
            selected: 0,
            enabled: true,
            id: None,
            action: None,
            attachments: Attachments::default(),
        }
    }

    pub fn style(mut self, style: PickerStyle) -> Self {
        self.style = style;
        self
    }

    /// Reads and writes the selected option index.
    pub fn bind(mut self, store: Store<usize>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, usize) + 'static) -> Self {
        self.action = Some(std::rc::Rc::new(action));
        self
    }
}

impl NativeView for Picker {
    fn attachments(&self) -> &Attachments {
        &self.attachments
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        &mut self.attachments
    }

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let action = self.action.clone();
        let on_change = move |cx: &mut Context<'_>, index: usize| {
            if let Some(action) = &action {
                action(cx, index);
            }
        };
        match self.style {
            PickerStyle::Menu => {
                let mut picker = zintl_ui_native::PopUpButton::new(self.options.clone())
                    .selected(self.selected)
                    .enabled(self.enabled)
                    .on_change(on_change);
                if let Some(store) = self.store {
                    picker = picker.bind(store);
                }
                if let Some(id) = &self.id {
                    picker = picker.id(id.clone());
                }
                picker.native_element(cx)
            }
            PickerStyle::Segmented => {
                let mut picker = zintl_ui_native::SegmentedControl::new(self.options.clone())
                    .selected(self.selected)
                    .enabled(self.enabled)
                    .on_change(on_change);
                if let Some(store) = self.store {
                    picker = picker.bind(store);
                }
                if let Some(id) = &self.id {
                    picker = picker.id(id.clone());
                }
                picker.native_element(cx)
            }
            PickerStyle::Radio => {
                let selected = self.store.map_or(self.selected, |store| *cx.get(store));
                let store = self.store;
                let on_change = std::rc::Rc::new(on_change);
                let radios = self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| {
                        let on_change = on_change.clone();
                        zintl_ui::element::ElementFactory::new(
                            zintl_ui_native::Button::radio(option.clone())
                                .on(index == selected)
                                .enabled(self.enabled)
                                .on_state_change(move |cx, on| {
                                    if !on {
                                        return;
                                    }
                                    if let Some(store) = store {
                                        cx.update(store, |selected| *selected = index);
                                    }
                                    on_change(cx, index);
                                }),
                        )
                    })
                    .collect::<ListFactory<RenderNode>>();
                let view =
                    zintl_ui_native::View::new(LayoutStyle::stack(Axis::Vertical, 6.0), radios);
                match &self.id {
                    Some(id) => view.id(id.clone()).native_element(cx),
                    None => view.native_element(cx),
                }
            }
        }
    }
}

/// Wraps a native control and forwards builder methods to it.
macro_rules! wrapper {
    ($(#[$meta:meta])* $name:ident($inner:ty)) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name {
            inner: $inner,
        }

        impl $name {
            pub fn id(mut self, id: impl Into<String>) -> Self {
                self.inner = self.inner.id(id);
                self
            }

            pub fn minimum_size(mut self, size: Size) -> Self {
                self.inner = self.inner.minimum_size(size);
                self
            }
        }

        impl NativeView for $name {
            fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
                self.inner.base_element(cx)
            }

            fn attachments(&self) -> &Attachments {
                self.inner.attachments()
            }

            fn attachments_mut(&mut self) -> &mut Attachments {
                self.inner.attachments_mut()
            }
        }
    };
}

wrapper!(
    /// An editable text field with suggested values.
    ComboBox(zintl_ui_native::ComboBox)
);

impl ComboBox {
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            inner: zintl_ui_native::ComboBox::new(items),
        }
    }

    pub fn bind(mut self, store: Store<String>) -> Self {
        self.inner = self.inner.bind(store);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.inner = self.inner.placeholder(placeholder);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.inner = self.inner.enabled(enabled);
        self
    }

    pub fn on_change(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, String) + 'static,
    ) -> Self {
        self.inner = self.inner.on_change(action);
        self
    }
}

wrapper!(
    /// A continuous slider over a closed range.
    Slider(zintl_ui_native::Slider)
);

impl Slider {
    pub fn new(range: RangeInclusive<f64>) -> Self {
        Self {
            inner: zintl_ui_native::Slider::new(*range.start(), *range.end()),
        }
    }

    pub fn bind(mut self, store: Store<f64>) -> Self {
        self.inner = self.inner.bind(store);
        self
    }

    /// Shows `count` tick marks below the track.
    pub fn ticks(mut self, count: usize) -> Self {
        self.inner = self.inner.tick_marks(count, false);
        self
    }

    /// Shows `count` tick marks and only allows their values.
    pub fn snapping_ticks(mut self, count: usize) -> Self {
        self.inner = self.inner.tick_marks(count, true);
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.inner = self.inner.fill_width();
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.inner = self.inner.enabled(enabled);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, f64) + 'static) -> Self {
        self.inner = self.inner.on_change(action);
        self
    }
}

wrapper!(
    /// A field that only accepts numbers, with a stepper.
    NumberField(zintl_ui_native::NumberField)
);

impl NumberField {
    pub fn new() -> Self {
        Self {
            inner: zintl_ui_native::NumberField::new(),
        }
    }

    pub fn bind(mut self, store: Store<f64>) -> Self {
        self.inner = self.inner.bind(store);
        self
    }

    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.inner = self.inner.range(*range.start(), *range.end());
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.inner = self.inner.step(step);
        self
    }

    /// Shows and accepts `digits` decimal places; zero accepts integers only.
    pub fn fraction_digits(mut self, digits: usize) -> Self {
        self.inner = self.inner.fraction_digits(digits);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.inner = self.inner.enabled(enabled);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, f64) + 'static) -> Self {
        self.inner = self.inner.on_change(action);
        self
    }
}

impl Default for NumberField {
    fn default() -> Self {
        Self::new()
    }
}

wrapper!(
    /// A search field that reports every edit.
    SearchField(zintl_ui_native::SearchField)
);

impl SearchField {
    pub fn new() -> Self {
        Self {
            inner: zintl_ui_native::SearchField::new(),
        }
    }

    pub fn bind(mut self, store: Store<String>) -> Self {
        self.inner = self.inner.bind(store);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.inner = self.inner.placeholder(placeholder);
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.inner = self.inner.fill_width();
        self
    }

    pub fn on_change(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, String) + 'static,
    ) -> Self {
        self.inner = self.inner.on_change(action);
        self
    }
}

impl Default for SearchField {
    fn default() -> Self {
        Self::new()
    }
}

wrapper!(
    /// A color swatch that edits its color with the system color panel.
    ColorWell(zintl_ui_native::ColorWell)
);

impl ColorWell {
    pub fn new() -> Self {
        Self {
            inner: zintl_ui_native::ColorWell::new(),
        }
    }

    pub fn bind(mut self, store: Store<Color>) -> Self {
        self.inner = self.inner.bind(store);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.inner = self.inner.enabled(enabled);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, Color) + 'static) -> Self {
        self.inner = self.inner.on_change(action);
        self
    }
}

impl Default for ColorWell {
    fn default() -> Self {
        Self::new()
    }
}

wrapper!(
    /// Scrolls its content when it does not fit; fills the remaining space by default.
    ScrollView(zintl_ui_native::ScrollView)
);

impl ScrollView {
    pub fn new<V>(content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            inner: zintl_ui_native::ScrollView::new(content),
        }
    }

    pub fn axes(mut self, axes: ScrollAxes) -> Self {
        self.inner = self.inner.axes(axes);
        self
    }

    /// Saves the scroll position and restores it when the view is shown again.
    pub fn bind(mut self, store: Store<ScrollStatus>) -> Self {
        self.inner = self.inner.bind(store);
        self
    }

    /// Uses a fixed height instead of filling the remaining space.
    pub fn height(mut self, height: f32) -> Self {
        self.inner = self.inner.height(height);
        self
    }
}

/// One page of a [`TabView`].
pub type Tab = zintl_ui_native::TabViewItem;

wrapper!(
    /// Native tabs that show one page at a time.
    TabView(zintl_ui_native::TabView)
);

impl TabView {
    pub fn new(tabs: impl IntoIterator<Item = Tab>) -> Self {
        Self {
            inner: zintl_ui_native::TabView::new(tabs),
        }
    }

    /// Reads and writes the selected tab index.
    pub fn bind(mut self, store: Store<usize>) -> Self {
        self.inner = self.inner.bind(store);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, usize) + 'static) -> Self {
        self.inner = self.inner.on_change(action);
        self
    }
}
