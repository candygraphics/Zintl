use std::cell::Cell;
use std::sync::Arc;

use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AsView, Button, CanvasView, ColorWell, ComboBox, NumberField, PopUpButton, ScrollView,
    SearchField, SegmentedControl, Slider, Switch, TabView, TextField, UiError, View, ViewRef,
};

use zintl_ui_layout::Size;
use zpd_appkit::geometry::Rect;

use super::canvas::CanvasBridge;
use crate::convert::Native;
use crate::{ButtonType, MainTaskSender, ViewKind};

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
    /// The offset waits until layout has sized the document.
    ScrollView(ScrollView, Cell<Option<(f64, f64)>>),
    TabView(TabView),
    Canvas(CanvasView),
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
            Self::ScrollView(view, _) => view.as_view(),
            Self::TabView(view) => view.as_view(),
            Self::Canvas(view) => view.as_view(),
        }
    }

    /// Creates a native view and applies every property of `kind`.
    ///
    /// `main_tasks` lets canvas invalidators reach the main thread.
    pub(super) fn create(
        application: &Application,
        kind: &ViewKind,
        main_tasks: Arc<MainTaskSender>,
    ) -> Result<Self, UiError> {
        let native = match kind {
            ViewKind::Container => {
                Self::Container(View::new(application, Rect::new(0.0, 0.0, 0.0, 0.0))?)
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
            ViewKind::ScrollView { .. } => {
                Self::ScrollView(ScrollView::new(application)?, Cell::new(None))
            }
            ViewKind::TabView { .. } => Self::TabView(TabView::new(application)?),
            ViewKind::Canvas2D { canvas, .. } => {
                let view = CanvasView::new(application)?;
                CanvasBridge::install(&view, canvas, main_tasks);
                Self::Canvas(view)
            }
        };
        native.apply(None, kind)?;
        Ok(native)
    }

    /// Raises declared minimums to the size the control's content needs.
    ///
    /// Views declare estimates; AppKit measures the actual title and font.
    /// Line-limited labels keep their declared width so they can truncate.
    pub(super) fn minimum_size(&self, kind: &ViewKind, declared: Size) -> Result<Size, UiError> {
        let (measure_width, measure_height) = match kind {
            ViewKind::Button(_)
            | ViewKind::Switch { .. }
            | ViewKind::PopUpButton { .. }
            | ViewKind::SegmentedControl { .. } => (true, true),
            // Wrapping labels need a width before they have a height.
            ViewKind::TextField {
                editable: false,
                multiline: false,
                style,
                ..
            } => (style.line_limit.is_none(), true),
            _ => (false, false),
        };
        if !measure_width && !measure_height {
            return Ok(declared);
        }
        let (width, height) = self.as_view().intrinsic_content_size()?;
        let raise = |declared: f32, measure: bool, size: Option<f64>| match (measure, size) {
            (true, Some(size)) => declared.max(size.ceil() as f32),
            _ => declared,
        };
        Ok(Size::new(
            raise(declared.width, measure_width, width),
            raise(declared.height, measure_height, height),
        ))
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
                    native.set_button_type(button.button_type.native())?;
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
                            native.set_bezel_style(style.native())?;
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
                    style,
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
                let old_style = match old {
                    Some(ViewKind::TextField {
                        multiline, style, ..
                    }) => Some((multiline, style)),
                    _ => None,
                };
                if old_style.is_none_or(|(old_multiline, old)| {
                    old_multiline != multiline || old.line_limit != style.line_limit
                }) {
                    field.set_multiline(*multiline)?;
                    if let Some(lines) = style.line_limit {
                        field.set_line_limit(lines)?;
                    }
                }
                let old_style = old_style.map(|(_, style)| style);
                if old_style.is_none_or(|old| old.font != style.font) {
                    field.set_font(style.font.as_ref().map(Native::native).as_ref())?;
                }
                if old_style.is_none_or(|old| old.color != style.color) {
                    field.set_text_color(style.color.native())?;
                }
                if old_style.is_none_or(|old| old.alignment != style.alignment) {
                    field.set_alignment(style.alignment.native())?;
                }
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
                    field.set_format(format.native())?;
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
                    well.set_color(color.native())?;
                }
            }
            (Self::ScrollView(scroll, restore), ViewKind::ScrollView { axes, offset }) => {
                if !matches!(old, Some(ViewKind::ScrollView { axes: old, .. }) if old == axes) {
                    scroll.set_axes(axes.native())?;
                }
                // Later offsets echo the user's scrolling, so only a new view restores one.
                if old.is_none() {
                    restore.set(*offset);
                }
            }
            (Self::TabView(tabs), ViewKind::TabView { selected, .. }) => {
                // Titles are applied with the pages while attaching children.
                tabs.select(*selected)?;
            }
            (
                Self::Canvas(view),
                ViewKind::Canvas2D {
                    redraw, focusable, ..
                },
            ) => {
                let (old_redraw, old_focusable) = match old {
                    Some(ViewKind::Canvas2D {
                        redraw, focusable, ..
                    }) => (Some(redraw), Some(focusable)),
                    _ => (None, None),
                };
                if old_redraw != Some(redraw) {
                    view.set_redraw(redraw.native())?;
                }
                if old_focusable != Some(focusable) {
                    view.set_focusable(*focusable);
                }
            }
            _ => return Err(UiError::InvalidHierarchy),
        }
        Ok(())
    }
}
