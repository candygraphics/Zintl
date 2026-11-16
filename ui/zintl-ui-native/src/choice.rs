//! Controls that choose between states or options.

use std::rc::Rc;

use zintl_ui::element::ElementFactory;
use zintl_ui::store::Store;
use zintl_ui_layout::{Axis, LayoutStyle, MainAxisDistribution, Size};

use crate::modifiers;
use crate::{
    Attach, Button, ButtonType, Change, ChangeRoute, Container, Context, Control, Element, Layout,
    ListFactory, NativeView, PopUpButton, RenderNode, SegmentedControl, Switch, Text,
    ViewModifiers, list,
};

native_view!(Toggle, Picker);

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
    action: Option<Rc<Change<bool>>>,
    modifiers: ViewModifiers,
}

impl Toggle {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            style: ToggleStyle::Switch,
            store: None,
            on: false,
            action: None,
            modifiers: ViewModifiers::default(),
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

    pub fn value(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, bool) + 'static) -> Self {
        self.action = Some(ChangeRoute::copied(action));
        self
    }

    fn route(&self) -> ChangeRoute<bool> {
        ChangeRoute::new(self.store, self.action.clone())
    }

    fn button(&self, button: Button) -> Button {
        let button = button
            .on(self.on)
            .enabled(self.modifiers.enabled())
            .route_state(self.route());
        match self.modifiers.id() {
            Some(id) => button.id(id),
            None => button,
        }
    }
}

impl NativeView for Toggle {
    modifiers!();

    // The switch or button carries the ID, not the labeled row.
    fn places_id(&self) -> bool {
        true
    }

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        match self.style {
            ToggleStyle::Checkbox => self
                .button(Button::checkbox(self.label.clone()))
                .native_element(cx),
            ToggleStyle::Button => {
                let width = self.label.chars().count() as f32 * 7.0 + 32.0;
                self.button(
                    Button::new(self.label.clone())
                        .button_type(ButtonType::PushOnPushOff)
                        .minimum_size(Size::new(width.max(80.0), 32.0)),
                )
                .native_element(cx)
            }
            ToggleStyle::Switch => {
                let mut switch = Switch::new()
                    .value(self.on)
                    .enabled(self.modifiers.enabled())
                    .route(self.route());
                if let Some(id) = self.modifiers.id() {
                    switch = switch.id(id);
                }
                if self.label.is_empty() {
                    return switch.native_element(cx);
                }
                // The label leads and the switch trails, as in System Settings.
                let mut row = LayoutStyle::stack(Axis::Horizontal, 12.0);
                row.main_axis_distribution = MainAxisDistribution::SpaceBetween;
                Container::new(row, list![Text::new(self.label.clone()), switch,])
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
    action: Option<Rc<Change<usize>>>,
    modifiers: ViewModifiers,
}

impl Picker {
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            style: PickerStyle::Menu,
            store: None,
            selected: 0,
            action: None,
            modifiers: ViewModifiers::default(),
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

    /// Selects the option at `index`.
    pub fn selection(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, usize) + 'static) -> Self {
        self.action = Some(ChangeRoute::copied(action));
        self
    }
}

impl NativeView for Picker {
    modifiers!();

    // The native picker or the radio group carries the ID.
    fn places_id(&self) -> bool {
        true
    }

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let route = ChangeRoute::new(self.store, self.action.clone());
        match self.style {
            PickerStyle::Menu => {
                let mut picker = PopUpButton::new(self.options.clone())
                    .selection(self.selected)
                    .enabled(self.modifiers.enabled())
                    .route(route);
                if let Some(id) = self.modifiers.id() {
                    picker = picker.id(id);
                }
                picker.native_element(cx)
            }
            PickerStyle::Segmented => {
                let mut picker = SegmentedControl::new(self.options.clone())
                    .selection(self.selected)
                    .enabled(self.modifiers.enabled())
                    .route(route);
                if let Some(id) = self.modifiers.id() {
                    picker = picker.id(id);
                }
                picker.native_element(cx)
            }
            PickerStyle::Radio => {
                let selected = self.store.map_or(self.selected, |store| *cx.get(store));
                let enabled = self.modifiers.enabled();
                let radios = self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| {
                        // Only the radio that turned on reports the selection.
                        let route = route.clone();
                        let turned_on = ChangeRoute::copied(move |cx, on| {
                            if on {
                                route.send(cx, index);
                            }
                        });
                        ElementFactory::new(
                            Button::radio(option.clone())
                                .on(index == selected)
                                .enabled(enabled)
                                .route_state(ChangeRoute::new(None, Some(turned_on))),
                        )
                    })
                    .collect::<ListFactory<RenderNode>>();
                let view = Container::new(LayoutStyle::stack(Axis::Vertical, 6.0), radios);
                match self.modifiers.id() {
                    Some(id) => view.id(id).native_element(cx),
                    None => view.native_element(cx),
                }
            }
        }
    }
}
