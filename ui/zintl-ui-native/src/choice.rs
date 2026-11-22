//! Controls that choose between states or options.

use std::rc::Rc;

use zintl_ui::element::ElementFactory;
use zintl_ui::store::Store;
use zintl_ui_layout::{Axis, LayoutStyle, MainAxisDistribution, Size};

use crate::modifiers;
use crate::{
    Attach, Button, ButtonType, Change, ChangeRoute, Container, Context, Control, Element, Layout,
    ListFactory, NativeView, PopUpButton, RenderNode, SegmentedControl, Switch, Text, TextEstimate,
    ViewModifiers, list,
};

native_view!(Toggle);

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
                let width = TextEstimate::width(&self.label, &TextEstimate::system_font()) + 32.0;
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

/// Selects one of several values, each shown with a label.
///
/// The shown value comes from `bind`, then `selection`, and otherwise is the
/// first option. A value that no option has shows nothing selected.
#[derive(Clone)]
pub struct Picker<V: Clone + PartialEq + 'static = usize> {
    options: Vec<(V, String)>,
    style: PickerStyle,
    store: Option<Store<V>>,
    selection: Option<V>,
    action: Option<Rc<Change<V>>>,
    modifiers: ViewModifiers,
}

impl<V: Clone + PartialEq + 'static> Picker<V> {
    /// Creates a picker from values and their labels, in display order.
    pub fn new<L: Into<String>>(options: impl IntoIterator<Item = (V, L)>) -> Self {
        let options: Vec<(V, String)> = options
            .into_iter()
            .map(|(value, label)| (value, label.into()))
            .collect();
        debug_assert!(
            options
                .iter()
                .enumerate()
                .all(|(index, (value, _))| !options[..index]
                    .iter()
                    .any(|(other, _)| other == value)),
            "picker options must have distinct values"
        );
        Self {
            options,
            style: PickerStyle::Menu,
            store: None,
            selection: None,
            action: None,
            modifiers: ViewModifiers::default(),
        }
    }

    pub fn style(mut self, style: PickerStyle) -> Self {
        self.style = style;
        self
    }

    /// Reads and writes the selected value.
    pub fn bind(mut self, store: Store<V>) -> Self {
        self.store = Some(store);
        self
    }

    /// Selects the option with `value`.
    pub fn selection(mut self, value: V) -> Self {
        self.selection = Some(value);
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, &V) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Returns the index of the option to show as selected.
    fn selected_index(&self, cx: &mut Context<'_>) -> Option<usize> {
        let value = match self.store {
            Some(store) => cx.get(store).clone(),
            None => match &self.selection {
                Some(value) => value.clone(),
                None => return (!self.options.is_empty()).then_some(0),
            },
        };
        self.options.iter().position(|(option, _)| *option == value)
    }

    /// Turns the option indexes the native controls report into values.
    fn index_route(&self) -> ChangeRoute<usize> {
        ChangeRoute::new(self.store, self.action.clone())
            .by_index(self.options.iter().map(|(value, _)| value.clone()))
    }

    fn labels(&self) -> Vec<String> {
        self.options
            .iter()
            .map(|(_, label)| label.clone())
            .collect()
    }
}

impl Picker<usize> {
    /// Creates a picker that selects its labels by index.
    pub fn indexed<L: Into<String>>(labels: impl IntoIterator<Item = L>) -> Self {
        Self::new(labels.into_iter().enumerate())
    }
}

impl<V: Clone + PartialEq + 'static> NativeView for Picker<V> {
    modifiers!();

    // The native picker or the radio group carries the ID.
    fn places_id(&self) -> bool {
        true
    }

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let selected = self.selected_index(cx);
        let route = self.index_route();
        match self.style {
            PickerStyle::Menu => {
                let mut picker = PopUpButton::new(self.labels())
                    .selection(selected)
                    .enabled(self.modifiers.enabled())
                    .route(route);
                if let Some(id) = self.modifiers.id() {
                    picker = picker.id(id);
                }
                picker.native_element(cx)
            }
            PickerStyle::Segmented => {
                let mut picker = SegmentedControl::new(self.labels())
                    .selection(selected)
                    .enabled(self.modifiers.enabled())
                    .route(route);
                if let Some(id) = self.modifiers.id() {
                    picker = picker.id(id);
                }
                picker.native_element(cx)
            }
            PickerStyle::Radio => {
                let enabled = self.modifiers.enabled();
                let radios = self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, (_, label))| {
                        // Only the radio that turned on reports the selection.
                        let route = route.clone();
                        let turned_on = ChangeRoute::copied(move |cx, on| {
                            if on {
                                route.send(cx, index);
                            }
                        });
                        ElementFactory::new(
                            Button::radio(label.clone())
                                .on(selected == Some(index))
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

impl<V: Clone + PartialEq + 'static> zintl_ui::view::View for Picker<V> {
    type Output = RenderNode;

    fn render(
        &self,
        cx: &mut Context<'_>,
    ) -> impl zintl_ui::element::IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
