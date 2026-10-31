use std::cell::{Cell, RefCell};
use std::rc::Rc;

use messageloop_dispatcherqueue::Sender;
use zintl_ui::event::EventRouteId;
use zintl_ui_backend_core::layout::{HostLayout, LayoutNative, LayoutSnapshot};
use zintl_ui_backend_core::tree::ErrorSlot;
use zintl_ui_backend_core::{BezelStyle, ButtonType, Color, Event, ViewKind};
use zintl_ui_layout::{LayoutStyle, Rect as LayoutRect, Size};
use zpd_winui3::{
    AsElement, BoolProperty, Control, ControlEvent, ControlKind, NumberProperty, TextProperty,
};

use super::AppError;
use super::app::{Message, MessageSender};

/// The route a native control reports to; it changes when the Composer remounts.
pub(super) type RouteCell = Rc<Cell<Option<EventRouteId>>>;

/// What a Canvas lays out when WinUI resizes it.
pub(super) enum HostContent {
    /// Top-level views of a window or popover.
    Host(HostLayout<NativeNode>),
    /// The children of a tab page, laid out with the page's own style.
    Page {
        style: LayoutStyle,
        children: Vec<LayoutSnapshot<NativeNode>>,
    },
}

/// Lays out a Canvas's content at its last known size.
pub(super) struct CanvasHost {
    content: RefCell<Option<HostContent>>,
    size: Cell<Option<Size>>,
    errors: ErrorSlot<AppError>,
}

impl CanvasHost {
    pub(super) fn new(errors: ErrorSlot<AppError>) -> Rc<Self> {
        Rc::new(Self {
            content: RefCell::new(None),
            size: Cell::new(None),
            errors,
        })
    }

    /// Replaces the content and lays it out when the size is known.
    pub(super) fn set_content(&self, content: HostContent) {
        *self.content.borrow_mut() = Some(content);
        if let Some(size) = self.size.get() {
            self.layout(size);
        }
    }

    pub(super) fn resize(&self, size: Size) {
        self.size.set(Some(size));
        self.layout(size);
    }

    fn layout(&self, size: Size) {
        // A resize during this layout is applied by the next one.
        let Ok(content) = self.content.try_borrow() else {
            return;
        };
        let result = match content.as_ref() {
            // TODO: Pass the title bar height once WinUI reports it; see TODO.md.
            Some(HostContent::Host(layout)) => layout.layout(size, 0.0),
            Some(HostContent::Page { style, children }) => {
                LayoutSnapshot::apply_children(*style, children, size)
            }
            None => Ok(()),
        };
        if let Err(error) = result {
            self.errors.record(error);
        }
    }
}

/// A WinUI control created for a [`ViewKind`].
pub(super) struct NativeNode {
    pub(super) control: Control,
    pub(super) route: RouteCell,
    /// Lays out tab-page children when this node is a Canvas.
    pub(super) host: Option<Rc<CanvasHost>>,
    /// The offset waits until layout has sized the scroll document.
    restore: Cell<Option<(f64, f64)>>,
}

impl NativeNode {
    /// Returns the control kind used for `kind`.
    pub(super) fn control_kind(kind: &ViewKind) -> ControlKind {
        match kind {
            ViewKind::Container => ControlKind::Canvas,
            ViewKind::Button(button) => match button.button_type {
                ButtonType::MomentaryPushIn => ControlKind::Button,
                ButtonType::PushOnPushOff => ControlKind::ToggleButton,
                ButtonType::Checkbox => ControlKind::CheckBox,
                ButtonType::Radio => ControlKind::RadioButton,
            },
            ViewKind::TextField { editable: true, .. } | ViewKind::TextEditor { .. } => {
                ControlKind::TextBox
            }
            ViewKind::TextField { .. } => ControlKind::TextBlock,
            ViewKind::Switch { .. } => ControlKind::ToggleSwitch,
            ViewKind::Slider { .. } => ControlKind::Slider,
            ViewKind::PopUpButton { .. } => ControlKind::ComboBox,
            ViewKind::SegmentedControl { .. } => ControlKind::SelectorBar,
            ViewKind::ComboBox { .. } => ControlKind::EditableComboBox,
            ViewKind::NumberField { .. } => ControlKind::NumberBox,
            ViewKind::SearchField { .. } => ControlKind::AutoSuggestBox,
            ViewKind::ColorWell { .. } => ControlKind::ColorButton,
            // TODO: Use a ListView with selection; see TODO.md.
            ViewKind::ScrollView { .. } | ViewKind::List { .. } => ControlKind::ScrollViewer,
            ViewKind::TabView { .. } => ControlKind::TabView,
            // TODO: Draw with Direct2D on a SwapChainPanel; see TODO.md.
            ViewKind::Canvas2D { .. } => ControlKind::Canvas,
        }
    }

    /// Returns whether this control can display `kind` without being recreated.
    pub(super) fn matches(&self, kind: &ViewKind) -> bool {
        self.control.kind() == Self::control_kind(kind)
    }

    /// Raises declared minimums to the sizes compact WinUI templates need.
    ///
    /// Declared sizes follow AppKit's 13-point system font, which is narrower
    /// than WinUI's 14-pixel Segoe UI.
    pub(super) fn minimum_size(kind: &ViewKind, declared: Size) -> Size {
        let text = |value: &str| value.chars().count() as f32 * Self::CHARACTER_WIDTH;
        let longest = |items: &[String]| items.iter().map(|item| text(item)).fold(0.0, f32::max);
        let (width, height) = match kind {
            ViewKind::Button(button) if button.bezel == Some(BezelStyle::Help) => (28.0, 28.0),
            ViewKind::Button(button) => {
                let icon = if button.system_image.is_some() {
                    24.0
                } else {
                    0.0
                };
                (text(&button.title) + icon + 24.0, 28.0)
            }
            ViewKind::TextField {
                editable: false,
                value,
                multiline: false,
                ..
            } => (text(value) + 8.0, 20.0),
            ViewKind::TextField { editable: true, .. }
            | ViewKind::Slider { .. }
            | ViewKind::ComboBox { .. }
            | ViewKind::SearchField { .. } => (0.0, 28.0),
            ViewKind::PopUpButton { items, .. } => (longest(items) + 64.0, 28.0),
            ViewKind::NumberField { .. } => (100.0, 28.0),
            ViewKind::Switch { .. } | ViewKind::ColorWell { .. } => (44.0, 28.0),
            ViewKind::SegmentedControl { segments, .. } => (
                segments.iter().map(|segment| text(segment) + 28.0).sum(),
                40.0,
            ),
            _ => (0.0, 0.0),
        };
        Size::new(declared.width.max(width), declared.height.max(height))
    }

    /// The average Segoe UI glyph width at 14 pixels.
    const CHARACTER_WIDTH: f32 = 8.0;

    /// Creates a control, applies every property of `kind`, and routes its events.
    pub(super) fn create(
        kind: &ViewKind,
        sender: &MessageSender,
        errors: &ErrorSlot<AppError>,
    ) -> Result<Self, AppError> {
        let control = Control::new(Self::control_kind(kind))?;
        let route = RouteCell::default();
        let host = matches!(kind, ViewKind::Container).then(|| CanvasHost::new(errors.clone()));
        let native = Self {
            control,
            route: route.clone(),
            host: host.clone(),
            restore: Cell::new(None),
        };
        native.apply(None, kind)?;
        let sender = sender.clone();
        native.control.set_event_handler(move |event| {
            if let ControlEvent::SizeChanged { width, height } = event {
                if let Some(host) = &host {
                    host.resize(Size::new(width as f32, height as f32));
                }
                return;
            }
            if let (Some(route), Some(event)) = (route.get(), Self::event(event)) {
                let _ = sender.send(Message::Event { route, event });
            }
        })?;
        Ok(native)
    }

    /// Converts a routed control event; size changes are handled locally.
    pub(super) fn event(event: ControlEvent) -> Option<Event> {
        Some(match event {
            ControlEvent::Clicked => Event::Activated,
            ControlEvent::TextChanged(value) => Event::TextChanged { value },
            ControlEvent::StateChanged(on) => Event::StateChanged { on },
            ControlEvent::ValueChanged(value) => Event::ValueChanged { value },
            ControlEvent::SelectionChanged(index) => Event::SelectionChanged { index },
            ControlEvent::ColorChanged(color) => Event::ColorChanged {
                color: Color::rgba(color.red, color.green, color.blue, color.alpha),
            },
            ControlEvent::Scrolled { x, y } => Event::Scrolled { x, y },
            ControlEvent::NavigationSelected(id) => Event::SidebarSelectionChanged { id },
            ControlEvent::SearchChanged(value) => Event::SidebarSearchChanged { value },
            ControlEvent::SizeChanged { .. } => return None,
        })
    }

    pub(super) fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        Ok(self.control.set_bool(BoolProperty::Enabled, enabled)?)
    }

    /// Applies `new`, skipping properties equal in `old`.
    ///
    /// User-editable values are compared with the native state instead, so a
    /// value the user just entered is not written back.
    pub(super) fn apply(&self, old: Option<&ViewKind>, new: &ViewKind) -> Result<(), AppError> {
        let control = &self.control;
        match new {
            ViewKind::Container => {}
            ViewKind::Button(button) => {
                let old = match old {
                    Some(ViewKind::Button(old)) => Some(old),
                    _ => None,
                };
                let changed = |field: fn(&zintl_ui_backend_core::Button) -> bool| {
                    old.is_none_or(|old| field(old) != field(button))
                };
                if old.is_none_or(|old| old.title != button.title) {
                    control.set_text(TextProperty::Text, &button.title)?;
                }
                if old.is_none_or(|old| old.system_image != button.system_image) {
                    let glyph = button.system_image.as_deref().and_then(Glyph::of);
                    control.set_text(TextProperty::Glyph, glyph.unwrap_or(""))?;
                }
                if old.is_none_or(|old| old.bezel != button.bezel) {
                    control.set_bool(BoolProperty::Borderless, button.bezel.is_none())?;
                    control.set_bool(BoolProperty::Help, button.bezel == Some(BezelStyle::Help))?;
                }
                if old.is_none_or(|old| old.key_equivalent != button.key_equivalent) {
                    let key = button.key_equivalent.as_deref();
                    control.set_bool(BoolProperty::DefaultAction, key == Some("\r"))?;
                    control.set_bool(BoolProperty::CancelAction, key == Some("\u{1b}"))?;
                }
                if changed(|button| button.destructive) {
                    control.set_bool(BoolProperty::Destructive, button.destructive)?;
                }
                if button.button_type != ButtonType::MomentaryPushIn
                    && control.bool(BoolProperty::Checked)? != button.on
                {
                    control.set_bool(BoolProperty::Checked, button.on)?;
                }
            }
            ViewKind::TextField {
                value,
                placeholder,
                editable,
                multiline,
                ..
            } => {
                control.set_text(TextProperty::Text, value)?;
                control.set_bool(BoolProperty::Multiline, *multiline)?;
                if *editable {
                    control.set_text(
                        TextProperty::Placeholder,
                        placeholder.as_deref().unwrap_or(""),
                    )?;
                }
            }
            ViewKind::TextEditor {
                value, editable, ..
            } => {
                // Only outside changes are written back, so typing keeps the caret.
                if control.text(TextProperty::Text)? != *value {
                    control.set_text(TextProperty::Text, value)?;
                }
                if old.is_none() {
                    control.set_bool(BoolProperty::Multiline, true)?;
                }
                // TODO: Apply the font and read-only state; see TODO.md.
                let _ = editable;
            }
            ViewKind::Switch { on } => {
                if control.bool(BoolProperty::Checked)? != *on || old.is_none() {
                    control.set_bool(BoolProperty::Checked, *on)?;
                }
            }
            ViewKind::Slider {
                value,
                minimum,
                maximum,
                tick_marks,
                snaps_to_ticks,
            } => {
                let unchanged = matches!(
                    old,
                    Some(ViewKind::Slider { minimum: a, maximum: b, tick_marks: c, snaps_to_ticks: d, .. })
                        if (a, b, c, d) == (minimum, maximum, tick_marks, snaps_to_ticks)
                );
                if !unchanged {
                    control.set_number(NumberProperty::Minimum, *minimum)?;
                    control.set_number(NumberProperty::Maximum, *maximum)?;
                    control.set_number(NumberProperty::TickCount, *tick_marks as f64)?;
                    control.set_bool(BoolProperty::SnapToTicks, *snaps_to_ticks)?;
                }
                if control.number(NumberProperty::Value)? != *value {
                    control.set_number(NumberProperty::Value, *value)?;
                }
            }
            ViewKind::PopUpButton { items, selected }
            | ViewKind::SegmentedControl {
                segments: items,
                selected,
            } => {
                let same_items = match (old, new) {
                    (Some(ViewKind::PopUpButton { items: old, .. }), _)
                    | (Some(ViewKind::SegmentedControl { segments: old, .. }), _) => old == items,
                    _ => false,
                };
                // Items come first so the selected index is always in range.
                if !same_items {
                    control.set_items(items)?;
                }
                let index = selected
                    .filter(|index| *index < items.len())
                    .map_or(-1.0, |index| index as f64);
                control.set_number(NumberProperty::SelectedIndex, index)?;
            }
            ViewKind::ComboBox {
                items,
                value,
                placeholder,
            } => {
                if !matches!(old, Some(ViewKind::ComboBox { items: old, .. }) if old == items) {
                    control.set_items(items)?;
                }
                control.set_text(
                    TextProperty::Placeholder,
                    placeholder.as_deref().unwrap_or(""),
                )?;
                control.set_text(TextProperty::Text, value)?;
            }
            ViewKind::NumberField { value, format } => {
                if !matches!(old, Some(ViewKind::NumberField { format: old, .. }) if old == format)
                {
                    control.set_number(NumberProperty::Minimum, format.minimum)?;
                    control.set_number(NumberProperty::Maximum, format.maximum)?;
                    control.set_number(NumberProperty::Step, format.step)?;
                    control.set_number(
                        NumberProperty::FractionDigits,
                        format.fraction_digits as f64,
                    )?;
                }
                if control.number(NumberProperty::Value)? != *value || old.is_none() {
                    control.set_number(NumberProperty::Value, *value)?;
                }
            }
            ViewKind::SearchField { value, placeholder } => {
                control.set_text(
                    TextProperty::Placeholder,
                    placeholder.as_deref().unwrap_or(""),
                )?;
                control.set_text(TextProperty::Text, value)?;
            }
            ViewKind::ColorWell { color } => {
                if !matches!(old, Some(ViewKind::ColorWell { color: old }) if old == color) {
                    control.set_color(zpd_winui3::Color {
                        red: color.red,
                        green: color.green,
                        blue: color.blue,
                        alpha: color.alpha,
                    })?;
                }
            }
            ViewKind::ScrollView { axes, offset } => {
                if !matches!(old, Some(ViewKind::ScrollView { axes: old, .. }) if old == axes) {
                    control.set_bool(BoolProperty::HorizontalScroll, axes.horizontal)?;
                    control.set_bool(BoolProperty::VerticalScroll, axes.vertical)?;
                }
                // Later offsets echo the user's scrolling, so only a new view restores one.
                if old.is_none() {
                    self.restore.set(*offset);
                }
            }
            ViewKind::List { .. } => {
                if old.is_none() {
                    control.set_bool(BoolProperty::HorizontalScroll, false)?;
                    control.set_bool(BoolProperty::VerticalScroll, true)?;
                }
            }
            ViewKind::TabView { titles, selected } => {
                if !matches!(old, Some(ViewKind::TabView { titles: old, .. }) if old == titles) {
                    control.set_items(titles)?;
                }
                control.set_number(NumberProperty::SelectedIndex, *selected as f64)?;
            }
        }
        Ok(())
    }
}

impl LayoutNative for NativeNode {
    type Error = AppError;

    fn apply_frame(&self, frame: LayoutRect, _parent_height: Option<f32>) -> Result<(), AppError> {
        Ok(self.control.set_frame(
            f64::from(frame.x),
            f64::from(frame.y),
            f64::from(frame.width),
            f64::from(frame.height),
        )?)
    }

    fn layout_hosted(
        &self,
        frame: LayoutRect,
        hosted: &[LayoutSnapshot<Self>],
    ) -> Result<(), AppError> {
        match self.control.kind() {
            ControlKind::ScrollViewer => {
                let (width, height) = (frame.width, frame.height);
                let size = HostLayout::place(
                    hosted,
                    0.0,
                    Some(width),
                    None,
                    Size::new(width, height),
                    None,
                )?;
                self.control.set_scroll_extent(
                    f64::from(size.width.max(width)),
                    f64::from(size.height.max(height)),
                )?;
                if let Some((x, y)) = self.restore.take() {
                    self.control.scroll_to(x, y)?;
                }
            }
            // Pages lay themselves out once WinUI sizes them inside the tab view.
            ControlKind::TabView => {
                for page in hosted {
                    if let Some(host) = &page.native.host {
                        host.set_content(HostContent::Page {
                            style: page.style,
                            children: page.children.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Segoe Fluent Icons glyphs for SF Symbol names.
pub(super) struct Glyph;

impl Glyph {
    pub(super) const TABLE: &[(&str, &str)] = &[
        ("cursorarrow.click", "\u{E7C9}"),
        ("envelope", "\u{E715}"),
        ("folder", "\u{E8B7}"),
        ("gearshape", "\u{E713}"),
        ("house", "\u{E80F}"),
        ("info.circle", "\u{E946}"),
        ("macwindow.on.rectangle", "\u{E8A7}"),
        ("magnifyingglass", "\u{E721}"),
        ("paintpalette", "\u{E790}"),
        ("person", "\u{E77B}"),
        ("plus", "\u{E710}"),
        ("questionmark.circle", "\u{E897}"),
        ("slider.horizontal.3", "\u{E9E9}"),
        ("square.and.arrow.down", "\u{E74E}"),
        ("square.and.arrow.up", "\u{E72D}"),
        ("square.split.2x1", "\u{E8A9}"),
        ("star", "\u{E734}"),
        ("trash", "\u{E74D}"),
    ];

    /// Returns the glyph for an SF Symbol name; unknown names show no icon.
    pub(super) fn of(name: &str) -> Option<&'static str> {
        Self::TABLE
            .binary_search_by(|(candidate, _)| (*candidate).cmp(name))
            .ok()
            .map(|index| Self::TABLE[index].1)
    }
}
