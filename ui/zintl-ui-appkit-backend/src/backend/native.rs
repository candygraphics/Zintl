use zpd_appkit::geometry::Rect as NativeRect;
use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AsView, Button, ColorWell, ComboBox, NumberField, PopUpButton, ScrollView, SearchField,
    SegmentedControl, Slider, Switch, TabView, TextField, UiError, View, ViewRef,
};

use crate::{ButtonType, ViewKind};

/// A native view created for a [`ViewKind`].
pub(super) enum NativeNode {
    Container(View),
    Button(Button),
    TextField(TextField),
    Switch(Switch),
    Slider(Slider),
    PopUpButton(PopUpButton),
    SegmentedControl(SegmentedControl),
    ComboBox(ComboBox),
    NumberField(NumberField),
    SearchField(SearchField),
    ColorWell(ColorWell),
    ScrollView(ScrollView),
    TabView(TabView),
}

impl NativeNode {
    pub(super) fn as_view(&self) -> ViewRef<'_> {
        match self {
            Self::Container(view) => view.as_view(),
            Self::Button(view) => view.as_view(),
            Self::TextField(view) => view.as_view(),
            Self::Switch(view) => view.as_view(),
            Self::Slider(view) => view.as_view(),
            Self::PopUpButton(view) => view.as_view(),
            Self::SegmentedControl(view) => view.as_view(),
            Self::ComboBox(view) => view.as_view(),
            Self::NumberField(view) => view.as_view(),
            Self::SearchField(view) => view.as_view(),
            Self::ColorWell(view) => view.as_view(),
            Self::ScrollView(view) => view.as_view(),
            Self::TabView(view) => view.as_view(),
        }
    }

    /// Creates a native view and applies every property of `kind`.
    pub(super) fn create(application: &Application, kind: &ViewKind) -> Result<Self, UiError> {
        let native = match kind {
            ViewKind::Container => {
                Self::Container(View::new(application, NativeRect::new(0.0, 0.0, 0.0, 0.0))?)
            }
            ViewKind::Button(button) => {
                Self::Button(Button::with_title(application, &button.title)?)
            }
            ViewKind::TextField { value, .. } => {
                Self::TextField(TextField::with_string(application, value)?)
            }
            ViewKind::Switch { .. } => Self::Switch(Switch::new(application)?),
            ViewKind::Slider { .. } => Self::Slider(Slider::new(application)?),
            ViewKind::PopUpButton { .. } => Self::PopUpButton(PopUpButton::new(application)?),
            ViewKind::SegmentedControl { .. } => {
                Self::SegmentedControl(SegmentedControl::new(application)?)
            }
            ViewKind::ComboBox { .. } => Self::ComboBox(ComboBox::new(application)?),
            ViewKind::NumberField { .. } => Self::NumberField(NumberField::new(application)?),
            ViewKind::SearchField { .. } => Self::SearchField(SearchField::new(application)?),
            ViewKind::ColorWell { .. } => Self::ColorWell(ColorWell::new(application)?),
            ViewKind::ScrollView { .. } => Self::ScrollView(ScrollView::new(application)?),
            ViewKind::TabView { .. } => Self::TabView(TabView::new(application)?),
        };
        native.apply(None, kind)?;
        Ok(native)
    }

    pub(super) fn set_enabled(&self, enabled: bool) -> Result<(), UiError> {
        match self {
            Self::NumberField(field) => field.set_enabled(enabled),
            _ => self.as_view().set_enabled(enabled),
        }
    }

    /// Applies `new`, skipping properties equal in `old`.
    ///
    /// User-editable values are compared with the native state instead, so a
    /// value the user just entered is not written back.
    pub(super) fn apply(&self, old: Option<&ViewKind>, new: &ViewKind) -> Result<(), UiError> {
        match (self, new) {
            (Self::Container(_), ViewKind::Container) => {}
            (Self::Button(native), ViewKind::Button(button)) => {
                let old = match old {
                    Some(ViewKind::Button(old)) => Some(old),
                    _ => None,
                };
                if old.is_none_or(|old| old.title != button.title) {
                    native.set_title(&button.title)?;
                }
                if old.is_none_or(|old| old.button_type != button.button_type) {
                    native.set_button_type(button.button_type)?;
                }
                let two_state =
                    matches!(button.button_type, ButtonType::Checkbox | ButtonType::Radio);
                if !two_state
                    && old.is_none_or(|old| {
                        old.bezel != button.bezel || old.button_type != button.button_type
                    })
                {
                    match button.bezel {
                        Some(style) => {
                            native.set_bordered(true)?;
                            native.set_bezel_style(style)?;
                        }
                        None => native.set_bordered(false)?,
                    }
                }
                if button.button_type != ButtonType::MomentaryPushIn && native.is_on()? != button.on
                {
                    native.set_on(button.on)?;
                }
                if old.is_none_or(|old| {
                    old.system_image != button.system_image || old.title != button.title
                }) {
                    native.set_system_image(button.system_image.as_deref())?;
                }
                if old.is_none_or(|old| old.key_equivalent != button.key_equivalent) {
                    native.set_key_equivalent(button.key_equivalent.as_deref().unwrap_or(""))?;
                }
                if old.is_none_or(|old| old.destructive != button.destructive) {
                    native.set_destructive(button.destructive)?;
                }
            }
            (
                Self::TextField(field),
                ViewKind::TextField {
                    value,
                    placeholder,
                    editable,
                    selectable,
                    bordered,
                    draws_background,
                    multiline,
                },
            ) => {
                if field.string_value()? != *value {
                    field.set_string_value(value)?;
                }
                field.set_placeholder_string(placeholder.as_deref())?;
                field.set_editable(*editable)?;
                field.set_selectable(*selectable)?;
                field.set_bordered(*bordered)?;
                field.set_draws_background(*draws_background)?;
                field.set_multiline(*multiline)?;
            }
            (Self::Switch(switch), ViewKind::Switch { on }) => {
                if switch.is_on()? != *on {
                    switch.set_on(*on)?;
                }
            }
            (
                Self::Slider(slider),
                ViewKind::Slider {
                    value,
                    minimum,
                    maximum,
                    tick_marks,
                    snaps_to_ticks,
                },
            ) => {
                let (range, ticks) = match old {
                    Some(ViewKind::Slider {
                        minimum: old_minimum,
                        maximum: old_maximum,
                        tick_marks: old_ticks,
                        snaps_to_ticks: old_snaps,
                        ..
                    }) => (
                        (old_minimum, old_maximum) != (minimum, maximum),
                        (old_ticks, old_snaps) != (tick_marks, snaps_to_ticks),
                    ),
                    _ => (true, true),
                };
                if range {
                    slider.set_range(*minimum, *maximum)?;
                }
                if ticks {
                    slider.set_tick_marks(*tick_marks, *snaps_to_ticks)?;
                }
                if slider.value()? != *value {
                    slider.set_value(*value)?;
                }
            }
            (Self::PopUpButton(button), ViewKind::PopUpButton { items, selected }) => {
                if !matches!(old, Some(ViewKind::PopUpButton { items: old, .. }) if old == items) {
                    button.set_items(items)?;
                }
                if button.selected()? != *selected {
                    button.select(*selected)?;
                }
            }
            (
                Self::SegmentedControl(control),
                ViewKind::SegmentedControl { segments, selected },
            ) => {
                if !matches!(old, Some(ViewKind::SegmentedControl { segments: old, .. }) if old == segments)
                {
                    control.set_segments(segments)?;
                }
                if control.selected()? != *selected {
                    control.select(*selected)?;
                }
            }
            (
                Self::ComboBox(combo),
                ViewKind::ComboBox {
                    items,
                    value,
                    placeholder,
                },
            ) => {
                if !matches!(old, Some(ViewKind::ComboBox { items: old, .. }) if old == items) {
                    combo.set_items(items)?;
                }
                combo.set_placeholder(placeholder.as_deref())?;
                if combo.string_value()? != *value {
                    combo.set_string_value(value)?;
                }
            }
            (Self::NumberField(field), ViewKind::NumberField { value, format }) => {
                if !matches!(old, Some(ViewKind::NumberField { format: old, .. }) if old == format)
                {
                    field.set_format(*format)?;
                }
                if field.value()? != *value || old.is_none() {
                    field.set_value(*value)?;
                }
            }
            (Self::SearchField(field), ViewKind::SearchField { value, placeholder }) => {
                field.set_placeholder(placeholder.as_deref())?;
                if field.string_value()? != *value {
                    field.set_string_value(value)?;
                }
            }
            (Self::ColorWell(well), ViewKind::ColorWell { color }) => {
                if !matches!(old, Some(ViewKind::ColorWell { color: old }) if old == color) {
                    well.set_color(*color)?;
                }
            }
            (Self::ScrollView(scroll), ViewKind::ScrollView { axes }) => {
                if !matches!(old, Some(ViewKind::ScrollView { axes: old }) if old == axes) {
                    scroll.set_axes(*axes)?;
                }
            }
            (Self::TabView(tabs), ViewKind::TabView { selected, .. }) => {
                // Titles are applied with the pages while attaching children.
                tabs.select(*selected)?;
            }
            _ => return Err(UiError::InvalidHierarchy),
        }
        Ok(())
    }
}
