use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AsView, Button, CanvasView, ColorWell, ComboBox, ListView, NumberField, PopUpButton,
    ProgressIndicator, ScrollView, SearchField, SegmentedControl, Separator, Slider, Switch,
    TabView, TextEditor, TextField, UiError, View, ViewRef,
};

use zintl_ui_layout::Size;
use zpd_appkit::geometry::Rect;

use zintl_ui_backend_core::layout::HostLayout;
use zintl_ui_backend_core::tree::ErrorSlot;

use super::canvas::CanvasBridge;
#[cfg(feature = "wgpu")]
use super::wgpu::WgpuBridge;
use super::{AppError, GpuSlot};
use crate::convert::{Native, Shared};
use crate::{ButtonType, MainTaskSender, ViewKind};

/// Scrolling that waits for a scroll view's next layout.
#[derive(Default)]
pub(super) struct ScrollState {
    /// The document point to restore once, after the view is created.
    pub(super) restore: Cell<Option<(f64, f64)>>,
    /// The descendant to scroll into view.
    pub(super) reveal: RefCell<Option<crate::ScrollTarget>>,
    /// Whether the content scrolls sideways, so it keeps its own width.
    pub(super) horizontal: Cell<bool>,
}

/// A native view created for a [`ViewKind`].
pub(super) enum NativeNode {
    Container(View),
    Button(Button),
    TextField(TextField),
    TextEditor(TextEditor),
    Switch(Switch),
    Slider(Slider),
    PopUpButton(PopUpButton),
    SegmentedControl(SegmentedControl),
    ComboBox(ComboBox),
    NumberField(NumberField),
    ProgressIndicator(ProgressIndicator),
    Separator(Separator),
    SearchField(SearchField),
    ColorWell(ColorWell),
    /// The offset waits until layout has sized the document.
    ScrollView(ScrollView, ScrollState),
    TabView(TabView),
    Canvas(CanvasView),
    #[cfg(feature = "wgpu")]
    WgpuCanvas(zpd_appkit::ui::MetalCanvasView),
    List(ListView, ListRows),
}

/// The layouts of a list's or table's cells; each lays out one cell at its bounds.
pub(super) struct ListRows {
    pub(super) layouts: RefCell<Vec<Rc<RefCell<HostLayout<NativeNode>>>>>,
    /// Rows lay out when AppKit displays them, outside the backend's update.
    pub(super) errors: ErrorSlot<AppError>,
}

impl NativeNode {
    pub(super) fn as_view(&self) -> ViewRef<'_> {
        match self {
            Self::Container(view) => view.as_view(),
            Self::Button(view) => view.as_view(),
            Self::TextField(view) => view.as_view(),
            Self::TextEditor(view) => view.as_view(),
            Self::Switch(view) => view.as_view(),
            Self::Slider(view) => view.as_view(),
            Self::PopUpButton(view) => view.as_view(),
            Self::SegmentedControl(view) => view.as_view(),
            Self::ComboBox(view) => view.as_view(),
            Self::NumberField(view) => view.as_view(),
            Self::ProgressIndicator(view) => view.as_view(),
            Self::Separator(view) => view.as_view(),
            Self::SearchField(view) => view.as_view(),
            Self::ColorWell(view) => view.as_view(),
            Self::ScrollView(view, _) => view.as_view(),
            Self::TabView(view) => view.as_view(),
            Self::Canvas(view) => view.as_view(),
            #[cfg(feature = "wgpu")]
            Self::WgpuCanvas(view) => view.as_view(),
            Self::List(view, _) => view.as_view(),
        }
    }

    /// Creates a native view and applies every property of `kind`.
    ///
    /// `main_tasks` lets canvas invalidators reach the main thread; `errors`
    /// receives failures of layout that AppKit starts later, such as list rows.
    pub(super) fn create(
        application: &Application,
        kind: &ViewKind,
        main_tasks: Arc<MainTaskSender>,
        gpu: &GpuSlot,
        errors: &ErrorSlot<AppError>,
    ) -> Result<Self, UiError> {
        #[cfg(not(feature = "wgpu"))]
        let _ = gpu;
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
            ViewKind::TextEditor { .. } => Self::TextEditor(TextEditor::new(application)?),
            ViewKind::Switch { .. } => Self::Switch(Switch::new(application)?),
            ViewKind::Slider { .. } => Self::Slider(Slider::new(application)?),
            ViewKind::PopUpButton { .. } => Self::PopUpButton(PopUpButton::new(application)?),
            ViewKind::SegmentedControl { .. } => {
                Self::SegmentedControl(SegmentedControl::new(application)?)
            }
            ViewKind::ComboBox { .. } => Self::ComboBox(ComboBox::new(application)?),
            ViewKind::NumberField { .. } => Self::NumberField(NumberField::new(application)?),
            ViewKind::ProgressIndicator { .. } => {
                Self::ProgressIndicator(ProgressIndicator::new(application)?)
            }
            ViewKind::Separator => Self::Separator(Separator::new(application)?),
            ViewKind::SearchField { .. } => Self::SearchField(SearchField::new(application)?),
            ViewKind::ColorWell { .. } => Self::ColorWell(ColorWell::new(application)?),
            ViewKind::ScrollView { .. } => {
                Self::ScrollView(ScrollView::new(application)?, ScrollState::default())
            }
            ViewKind::TabView { .. } => Self::TabView(TabView::new(application)?),
            ViewKind::List { .. } | ViewKind::Table { .. } => Self::List(
                ListView::new(application)?,
                ListRows {
                    layouts: RefCell::new(Vec::new()),
                    errors: errors.clone(),
                },
            ),
            ViewKind::Canvas2D { canvas, .. } => {
                let view = CanvasView::new(application)?;
                CanvasBridge::install(&view, canvas, main_tasks);
                Self::Canvas(view)
            }
            #[cfg(feature = "wgpu")]
            ViewKind::WgpuCanvas { canvas, .. } => {
                let view = zpd_appkit::ui::MetalCanvasView::new(application)?;
                WgpuBridge::install(&view, canvas, main_tasks, gpu, errors);
                Self::WgpuCanvas(view)
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

    /// Replaces a line-limited label's estimated content size with its text's measured size.
    ///
    /// Such labels may shrink below it and truncate, so it is their preferred
    /// size rather than a minimum.
    pub(super) fn content_size(
        &self,
        kind: &ViewKind,
        declared: Option<Size>,
    ) -> Result<Option<Size>, UiError> {
        let (
            ViewKind::TextField {
                editable: false,
                multiline: false,
                style,
                ..
            },
            Some(declared),
        ) = (kind, declared)
        else {
            return Ok(declared);
        };
        if style.line_limit.is_none() {
            return Ok(Some(declared));
        }
        let (width, _) = self.as_view().intrinsic_content_size()?;
        Ok(Some(Size::new(
            width.map_or(declared.width, |width| width.ceil() as f32),
            declared.height,
        )))
    }

    /// Sets the tooltip on the view the pointer rests on.
    pub(super) fn set_tool_tip(&self, text: Option<&str>) -> Result<(), UiError> {
        match self {
            Self::NumberField(field) => field.set_tool_tip(text),
            _ => self.as_view().set_tool_tip(text),
        }
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
                // A new title is colored again by `set_title`, so only a changed color is applied.
                if old.is_none_or(|old| old.title_color != button.title_color) {
                    native.set_title_color(button.title_color.map(|color| color.native()))?;
                }
                native.set_shows_menu(button.shows_menu);
                native.set_keeps_state(button.keeps_state);
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
                // Typed text stays until the value changes, so unbound fields keep edits.
                let changed =
                    !matches!(old, Some(ViewKind::TextField { value: old, .. }) if old == value);
                if changed && field.string_value()? != *value {
                    field.set_string_value(value)?;
                }
                // Reapplying unchanged properties while editing would clear the typing undo history.
                let old_field = match old {
                    Some(ViewKind::TextField {
                        placeholder,
                        editable,
                        selectable,
                        bordered,
                        draws_background,
                        ..
                    }) => Some((
                        placeholder,
                        editable,
                        selectable,
                        bordered,
                        draws_background,
                    )),
                    _ => None,
                };
                if old_field.is_none_or(|old| old.0 != placeholder) {
                    field.set_placeholder_string(placeholder.as_deref())?;
                }
                if old_field.is_none_or(|old| old.1 != editable) {
                    field.set_editable(*editable)?;
                }
                if old_field.is_none_or(|old| old.2 != selectable) {
                    field.set_selectable(*selectable)?;
                }
                if old_field.is_none_or(|old| old.3 != bordered) {
                    field.set_bordered(*bordered)?;
                }
                if old_field.is_none_or(|old| old.4 != draws_background) {
                    field.set_draws_background(*draws_background)?;
                }
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
            (Self::TextEditor(editor), ViewKind::TextEditor { text, focus }) => {
                let (old_text, old_focus) = match old {
                    Some(ViewKind::TextEditor { text, focus }) => (Some(text), Some(*focus)),
                    _ => (None, None),
                };
                // Each new focus request moves keyboard focus to the editor.
                if *focus > 0 && old_focus != Some(*focus) {
                    editor.focus()?;
                }
                // Text the editor already shows is not written back, so typing keeps
                // the insertion point. Unchanged text is skipped too: AppKit cannot
                // keep a paragraph style on an empty last line.
                if old_text != Some(text) && editor.rich_text()?.shared() != *text {
                    editor.set_rich_text(&text.native())?;
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
            (
                Self::NumberField(field),
                ViewKind::NumberField {
                    value,
                    format,
                    text,
                },
            ) => {
                if !matches!(old, Some(ViewKind::NumberField { format: old, .. }) if old == format)
                {
                    field.set_format(format.native())?;
                }
                if !matches!(old, Some(ViewKind::NumberField { text: old, .. }) if old == text) {
                    field.set_text(text.as_deref())?;
                }
                // Typed numbers stay until the value changes, so unbound fields keep edits.
                // Out-of-range input is always replaced, as it was clamped.
                let shown = field.value()?;
                let in_range = (format.minimum..=format.maximum).contains(&shown);
                match old {
                    Some(ViewKind::NumberField { value: old, .. }) if old == value && in_range => {}
                    Some(_) if shown == *value => {}
                    _ => field.set_value(*value)?,
                }
            }
            (Self::ProgressIndicator(indicator), ViewKind::ProgressIndicator { value }) => {
                if !matches!(old, Some(ViewKind::ProgressIndicator { value: old }) if old == value)
                {
                    indicator.set_value(*value)?;
                }
            }
            (Self::Separator(_), ViewKind::Separator) => {}
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
            (
                Self::ScrollView(scroll, state),
                ViewKind::ScrollView {
                    axes,
                    offset,
                    reveal,
                },
            ) => {
                if !matches!(old, Some(ViewKind::ScrollView { axes: old, .. }) if old == axes) {
                    scroll.set_axes(axes.native())?;
                    state.horizontal.set(axes.horizontal);
                }
                // Later offsets echo the user's scrolling, so only a new view restores one.
                if old.is_none() {
                    state.restore.set(*offset);
                }
                // The target scrolls into view once layout has placed it.
                if !matches!(old, Some(ViewKind::ScrollView { reveal: old, .. }) if old == reveal) {
                    state.reveal.replace(reveal.clone());
                }
            }
            (Self::TabView(tabs), ViewKind::TabView { selected, .. }) => {
                // Titles are applied with the pages while attaching children.
                tabs.select(*selected)?;
            }
            (
                Self::Canvas(view),
                ViewKind::Canvas2D {
                    redraw,
                    focusable,
                    reports_input,
                    ..
                },
            ) => {
                view.set_reports_input(*reports_input);
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
            #[cfg(feature = "wgpu")]
            (
                Self::WgpuCanvas(view),
                ViewKind::WgpuCanvas {
                    redraw,
                    focusable,
                    reports_input,
                    ..
                },
            ) => {
                view.set_reports_input(*reports_input);
                let (old_redraw, old_focusable) = match old {
                    Some(ViewKind::WgpuCanvas {
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
            (
                Self::List(list, _),
                ViewKind::List {
                    selected, style, ..
                },
            ) => {
                if !matches!(old, Some(ViewKind::List { style: old, .. }) if old == style) {
                    list.set_style(style.native())?;
                }
                // Rows are applied while attaching children; selection follows them.
                if list.selected_id()? != *selected {
                    list.select(selected.as_deref())?;
                    if let Some(id) = selected {
                        list.reveal(id)?;
                    }
                }
            }
            (
                Self::List(list, _),
                ViewKind::Table {
                    columns,
                    selected,
                    striped,
                    ..
                },
            ) => {
                if old.is_none() {
                    list.set_style(zpd_appkit::ui::ListStyle::Plain)?;
                }
                // New columns take effect when the rows are attached again.
                if !matches!(old, Some(ViewKind::Table { columns: old, .. }) if old == columns) {
                    list.set_columns(&columns.iter().map(Native::native).collect::<Vec<_>>())?;
                }
                if !matches!(old, Some(ViewKind::Table { striped: old, .. }) if old == striped) {
                    list.set_striped(*striped)?;
                }
                if list.selected_id()? != *selected {
                    list.select(selected.as_deref())?;
                    if let Some(id) = selected {
                        list.reveal(id)?;
                    }
                }
            }
            _ => return Err(UiError::InvalidHierarchy),
        }
        Ok(())
    }
}
