use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use messageloop_dispatcherqueue::Sender;
use zintl_ui::event::EventRouteId;
use zintl_ui_backend_core::graphics::{self, AttributedText, SpanStyle};
use zintl_ui_backend_core::layout::{HostLayout, LayoutNative, LayoutSnapshot};
use zintl_ui_backend_core::tree::ErrorSlot;
use zintl_ui_backend_core::{
    BezelStyle, ButtonType, Color, Event, MainTaskSender, RedrawPolicy, ScrollAlignment,
    ScrollTarget, TableColumn, TextAlignment, TextColor, TextStyle, ViewKind,
};
use zintl_ui_layout::{LayoutDimension, LayoutStyle, Rect as LayoutRect, Size};
use zpd_winui3::{
    AsElement, BoolProperty, CanvasMode, Control, ControlEvent, ControlKind, ElementRef,
    NumberProperty, ParagraphStyle, RichRun, RichText, TextProperty, TextTraits,
};

use super::app::{Message, MessageSender};
use super::canvas::{CanvasBridge, InputRoute};
#[cfg(feature = "wgpu")]
use super::wgpu::WgpuBridge;
use super::{AppError, GpuSlot};

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
    /// The height of the canvas's top part under the window's title bar.
    inset: Cell<f32>,
    errors: ErrorSlot<AppError>,
}

impl CanvasHost {
    pub(super) fn new(errors: ErrorSlot<AppError>) -> Rc<Self> {
        Rc::new(Self {
            content: RefCell::new(None),
            size: Cell::new(None),
            inset: Cell::new(0.0),
            errors,
        })
    }

    /// Replaces the content and lays it out when the size is known.
    pub(super) fn set_content(&self, content: HostContent) {
        *self.content.borrow_mut() = Some(content);
        self.relayout();
    }

    pub(super) fn resize(&self, size: Size, inset: f32) {
        self.size.set(Some(size));
        self.inset.set(inset);
        self.relayout();
    }

    /// Lays out again when the title bar starts or stops covering the canvas.
    pub(super) fn set_inset(&self, inset: f32) {
        if self.inset.replace(inset) != inset {
            self.relayout();
        }
    }

    fn relayout(&self) {
        let Some(size) = self.size.get() else {
            return;
        };
        // A resize during this layout is applied by the next one.
        let Ok(content) = self.content.try_borrow() else {
            return;
        };
        let result = match content.as_ref() {
            Some(HostContent::Host(layout)) => layout.layout(size, self.inset.get()),
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

/// What creating a native view needs from the backend.
pub(super) struct Environment<'a> {
    pub(super) sender: &'a MessageSender,
    pub(super) errors: &'a ErrorSlot<AppError>,
    /// Lets canvas invalidators reach the main thread.
    pub(super) main_tasks: Arc<MainTaskSender>,
    pub(super) gpu: &'a GpuSlot,
}

/// The WinUI element of a [`NativeNode`].
pub(super) enum NativeView {
    Control(Control),
    Canvas {
        canvas: zpd_winui3::Canvas,
        mode: CanvasMode,
        /// Whether input is reported as [`Event::Canvas`].
        reports_input: Rc<Cell<bool>>,
    },
}

impl AsElement for NativeView {
    fn as_element(&self) -> ElementRef<'_> {
        match self {
            Self::Control(control) => control.as_element(),
            Self::Canvas { canvas, .. } => canvas.as_element(),
        }
    }
}

/// A WinUI control or canvas created for a [`ViewKind`].
pub(super) struct NativeNode {
    pub(super) view: NativeView,
    pub(super) route: RouteCell,
    /// Lays out tab-page children when this node is a Canvas.
    pub(super) host: Option<Rc<CanvasHost>>,
    /// Whether clicks under the title bar reach this view.
    pub(super) title_bar_interactive: Cell<bool>,
    /// The offset waits until layout has sized the scroll document.
    restore: Cell<Option<(f64, f64)>>,
    /// The descendant to scroll into view after the next layout.
    reveal: RefCell<Option<ScrollTarget>>,
    /// Whether scroll content scrolls sideways, so it keeps its own width.
    horizontal: Cell<bool>,
    /// A table's columns; lists have none and one full-width cell per row.
    columns: RefCell<Vec<TableColumn>>,
    /// The column widths last shown in a table's header.
    header_widths: RefCell<Vec<f32>>,
}

impl NativeNode {
    /// Returns the control kind used for `kind`, or `None` for canvases.
    pub(super) fn control_kind(kind: &ViewKind) -> Option<ControlKind> {
        Some(match kind {
            ViewKind::Container => ControlKind::Canvas,
            ViewKind::Button(button) => match button.button_type {
                ButtonType::MomentaryPushIn => ControlKind::Button,
                ButtonType::PushOnPushOff => ControlKind::ToggleButton,
                ButtonType::Checkbox => ControlKind::CheckBox,
                ButtonType::Radio => ControlKind::RadioButton,
            },
            ViewKind::TextField { editable: true, .. } => ControlKind::TextBox,
            ViewKind::TextField { .. } => ControlKind::TextBlock,
            ViewKind::TextEditor { .. } => ControlKind::RichEditBox,
            ViewKind::Switch { .. } => ControlKind::ToggleSwitch,
            ViewKind::Slider { .. } => ControlKind::Slider,
            ViewKind::PopUpButton { .. } => ControlKind::ComboBox,
            ViewKind::SegmentedControl { .. } => ControlKind::SelectorBar,
            ViewKind::ComboBox { .. } => ControlKind::EditableComboBox,
            ViewKind::NumberField { .. } => ControlKind::NumberBox,
            ViewKind::SearchField { .. } => ControlKind::AutoSuggestBox,
            ViewKind::ColorWell { .. } => ControlKind::ColorButton,
            ViewKind::ProgressIndicator { value: None } => ControlKind::ProgressRing,
            ViewKind::ProgressIndicator { value: Some(_) } => ControlKind::ProgressBar,
            ViewKind::Separator => ControlKind::Separator,
            ViewKind::ScrollView { .. } => ControlKind::ScrollViewer,
            ViewKind::List { .. } | ViewKind::Table { .. } => ControlKind::ListView,
            ViewKind::TabView { .. } => ControlKind::TabView,
            ViewKind::Canvas2D { .. } => return None,
            #[cfg(feature = "wgpu")]
            ViewKind::WgpuCanvas { .. } => return None,
        })
    }

    fn canvas_mode(kind: &ViewKind) -> Option<CanvasMode> {
        match kind {
            ViewKind::Canvas2D { .. } => Some(CanvasMode::Direct2D),
            #[cfg(feature = "wgpu")]
            ViewKind::WgpuCanvas { .. } => Some(CanvasMode::External),
            _ => None,
        }
    }

    pub(super) fn control(&self) -> Option<&Control> {
        match &self.view {
            NativeView::Control(control) => Some(control),
            NativeView::Canvas { .. } => None,
        }
    }

    /// Returns whether this view can display `kind` without being recreated.
    pub(super) fn matches(&self, kind: &ViewKind) -> bool {
        match &self.view {
            NativeView::Control(control) => Some(control.kind()) == Self::control_kind(kind),
            NativeView::Canvas { mode, .. } => Some(*mode) == Self::canvas_mode(kind),
        }
    }

    /// Raises declared minimums to the sizes compact WinUI templates need.
    ///
    /// Declared sizes follow AppKit's 13-point system font, which is narrower
    /// than WinUI's 14-pixel Segoe UI. Labels are measured; line-limited ones
    /// keep their declared width so they can truncate.
    pub(super) fn minimum_size(&self, kind: &ViewKind, declared: Size) -> Size {
        Self::minimum_for(kind, declared, self.label_width(kind))
    }

    /// Like [`NativeNode::minimum_size`], with a label's `measured` text width.
    pub(super) fn minimum_for(kind: &ViewKind, declared: Size, measured: Option<f32>) -> Size {
        let text = Self::text_width;
        let longest = |items: &[String]| items.iter().map(|item| text(item)).fold(0.0, f32::max);
        let (width, height) = match kind {
            ViewKind::Button(button) if button.bezel == Some(BezelStyle::Help) => (28.0, 28.0),
            ViewKind::Button(button) => {
                let icon = if button.system_image.is_some() {
                    24.0
                } else {
                    0.0
                };
                // Template padding and border take 24 pixels; 4 more absorb rounding.
                (text(&button.title) + icon + 28.0, 28.0)
            }
            ViewKind::TextField {
                editable: false,
                multiline: false,
                style,
                value,
                ..
            } => {
                let measured = measured.unwrap_or_else(|| text(value) + 8.0);
                let width = if style.line_limit.is_none() {
                    measured
                } else {
                    0.0
                };
                (width, 20.0)
            }
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
            ViewKind::ProgressIndicator { value: None } => (16.0, 16.0),
            _ => (0.0, 0.0),
        };
        Size::new(declared.width.max(width), declared.height.max(height))
    }

    /// Replaces a line-limited label's estimated content size with its text's measured width.
    ///
    /// Such labels may shrink below it and truncate, so it is their preferred
    /// size rather than a minimum.
    pub(super) fn content_size(&self, kind: &ViewKind, declared: Option<Size>) -> Option<Size> {
        Self::content_for(kind, declared, self.label_width(kind))
    }

    /// Like [`NativeNode::content_size`], with a label's `measured` text width.
    pub(super) fn content_for(
        kind: &ViewKind,
        declared: Option<Size>,
        measured: Option<f32>,
    ) -> Option<Size> {
        match (kind, declared) {
            (
                ViewKind::TextField {
                    editable: false,
                    multiline: false,
                    style,
                    ..
                },
                Some(declared),
            ) if style.line_limit.is_some() => Some(Size::new(
                measured.unwrap_or(declared.width),
                declared.height,
            )),
            _ => declared,
        }
    }

    /// Measures a single-line label's text width with unbounded space.
    fn label_width(&self, kind: &ViewKind) -> Option<f32> {
        if !matches!(
            kind,
            ViewKind::TextField {
                editable: false,
                multiline: false,
                ..
            }
        ) {
            return None;
        }
        let (width, _) = self.view.measure(f64::INFINITY, f64::INFINITY).ok()?;
        (width > 0.0).then(|| width.ceil() as f32)
    }

    /// The average Segoe UI glyph width at 14 pixels, used when DirectWrite cannot measure.
    const CHARACTER_WIDTH: f32 = 8.0;

    /// Measures `value` in WinUI's 14-pixel control font.
    fn text_width(value: &str) -> f32 {
        let font = zpd_winui3::Font {
            family: Some("Segoe UI Variable Text"),
            size: 14.0,
            weight: 400,
            italic: false,
        };
        zpd_winui3::measure_text(value, &font)
            .map(|metrics| metrics.width.ceil() as f32)
            .unwrap_or_else(|_| value.chars().count() as f32 * Self::CHARACTER_WIDTH)
    }

    /// Creates a view, applies every property of `kind`, and routes its events.
    pub(super) fn create(kind: &ViewKind, environment: &Environment<'_>) -> Result<Self, AppError> {
        let route = RouteCell::default();
        let (view, host) = match (Self::control_kind(kind), kind.canvas()) {
            (None, Some(node)) => {
                let mode = Self::canvas_mode(kind).expect("canvases have a mode");
                let canvas = zpd_winui3::Canvas::new(mode)?;
                let input = InputRoute {
                    reports_input: Rc::new(Cell::new(false)),
                    route: route.clone(),
                    sender: environment.sender.clone(),
                };
                let reports_input = input.reports_input.clone();
                match mode {
                    CanvasMode::Direct2D => {
                        CanvasBridge::install(&canvas, node, input, environment.main_tasks.clone())?
                    }
                    #[cfg(feature = "wgpu")]
                    CanvasMode::External => WgpuBridge::install(
                        &canvas,
                        node,
                        input,
                        environment.main_tasks.clone(),
                        environment.gpu,
                        environment.errors,
                    )?,
                    #[cfg(not(feature = "wgpu"))]
                    CanvasMode::External => {
                        let _ = environment.gpu;
                    }
                }
                let view = NativeView::Canvas {
                    canvas,
                    mode,
                    reports_input,
                };
                (view, None)
            }
            (control_kind, _) => {
                let control_kind = control_kind.expect("views other than canvases are controls");
                let control = Control::new(control_kind)?;
                let host = matches!(kind, ViewKind::Container)
                    .then(|| CanvasHost::new(environment.errors.clone()));
                let (event_host, event_route) = (host.clone(), route.clone());
                let sender = environment.sender.clone();
                control.set_event_handler(move |event| {
                    if let ControlEvent::SizeChanged {
                        width,
                        height,
                        inset,
                    } = event
                    {
                        if let Some(host) = &event_host {
                            host.resize(Size::new(width as f32, height as f32), inset as f32);
                        }
                        return;
                    }
                    if let (Some(route), Some(event)) = (event_route.get(), Self::event(event)) {
                        let _ = sender.send(Message::Event { route, event });
                    }
                })?;
                (NativeView::Control(control), host)
            }
        };
        let native = Self {
            view,
            route,
            host,
            title_bar_interactive: Cell::new(false),
            restore: Cell::new(None),
            reveal: RefCell::new(None),
            horizontal: Cell::new(false),
            columns: RefCell::new(Vec::new()),
            header_widths: RefCell::new(Vec::new()),
        };
        native.apply(None, kind)?;
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
            ControlEvent::RichTextChanged(text) => Event::RichTextChanged {
                text: Self::attributed(&text),
            },
            ControlEvent::ListSelectionChanged(id) => Event::ListSelectionChanged { id },
            ControlEvent::MenuSelected(id) => Event::MenuItemSelected { id },
            ControlEvent::TextCommitted(value) => Event::TextCommitted { value },
            ControlEvent::SizeChanged { .. } => return None,
        })
    }

    /// Converts attributed text to a RichEditBox document.
    pub(super) fn rich_text(text: &AttributedText) -> RichText {
        RichText {
            runs: text
                .runs()
                .map(|(text, style)| RichRun {
                    text: text.to_owned(),
                    traits: TextTraits {
                        bold: style.bold,
                        italic: style.italic,
                        underline: style.underline,
                        strikethrough: style.strikethrough,
                        monospace: style.monospace,
                    },
                    link: style.link.clone(),
                })
                .collect(),
            paragraphs: text
                .paragraphs()
                .iter()
                .map(|style| match style {
                    graphics::ParagraphStyle::Title => ParagraphStyle::Title,
                    graphics::ParagraphStyle::Heading => ParagraphStyle::Heading,
                    graphics::ParagraphStyle::Subheading => ParagraphStyle::Subheading,
                    graphics::ParagraphStyle::Body => ParagraphStyle::Body,
                    graphics::ParagraphStyle::Monospaced => ParagraphStyle::Monospaced,
                    graphics::ParagraphStyle::Bulleted => ParagraphStyle::Bulleted,
                    graphics::ParagraphStyle::Dashed => ParagraphStyle::Dashed,
                    graphics::ParagraphStyle::Numbered => ParagraphStyle::Numbered,
                    graphics::ParagraphStyle::Checklist { done: false } => {
                        ParagraphStyle::Checklist
                    }
                    graphics::ParagraphStyle::Checklist { done: true } => {
                        ParagraphStyle::ChecklistDone
                    }
                })
                .collect(),
        }
    }

    /// Converts a RichEditBox document to attributed text.
    pub(super) fn attributed(text: &RichText) -> AttributedText {
        let mut attributed = AttributedText::new();
        for run in &text.runs {
            attributed.push(
                &run.text,
                SpanStyle {
                    bold: run.traits.bold,
                    italic: run.traits.italic,
                    underline: run.traits.underline,
                    strikethrough: run.traits.strikethrough,
                    monospace: run.traits.monospace,
                    link: run.link.clone(),
                },
            );
        }
        for (index, style) in text.paragraphs.iter().enumerate() {
            let style = match style {
                ParagraphStyle::Title => graphics::ParagraphStyle::Title,
                ParagraphStyle::Heading => graphics::ParagraphStyle::Heading,
                ParagraphStyle::Subheading => graphics::ParagraphStyle::Subheading,
                ParagraphStyle::Body => graphics::ParagraphStyle::Body,
                ParagraphStyle::Monospaced => graphics::ParagraphStyle::Monospaced,
                ParagraphStyle::Bulleted => graphics::ParagraphStyle::Bulleted,
                ParagraphStyle::Dashed => graphics::ParagraphStyle::Dashed,
                ParagraphStyle::Numbered => graphics::ParagraphStyle::Numbered,
                ParagraphStyle::Checklist => graphics::ParagraphStyle::Checklist { done: false },
                ParagraphStyle::ChecklistDone => graphics::ParagraphStyle::Checklist { done: true },
            };
            attributed.set_paragraph(index, style);
        }
        attributed
    }

    pub(super) fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        match &self.view {
            NativeView::Control(control) => control.set_bool(BoolProperty::Enabled, enabled)?,
            // Canvases have no disabled look; they keep handling input.
            NativeView::Canvas { .. } => {}
        }
        Ok(())
    }

    /// Converts a declared text style.
    fn text_style(style: &TextStyle) -> zpd_winui3::TextStyle<'_> {
        zpd_winui3::TextStyle {
            font: style.font.as_ref().map(|font| zpd_winui3::Font {
                family: font.family.as_deref(),
                size: font.size(),
                weight: font.weight.0,
                italic: font.italic,
            }),
            color: match style.color {
                TextColor::Primary => zpd_winui3::TextColor::Primary,
                TextColor::Secondary => zpd_winui3::TextColor::Secondary,
                TextColor::Tertiary => zpd_winui3::TextColor::Tertiary,
                TextColor::Accent => zpd_winui3::TextColor::Accent,
                TextColor::Custom(color) => zpd_winui3::TextColor::Custom(Self::color(color)),
            },
            alignment: match style.alignment {
                TextAlignment::Leading => zpd_winui3::TextStyleAlignment::Leading,
                TextAlignment::Center => zpd_winui3::TextStyleAlignment::Center,
                TextAlignment::Trailing => zpd_winui3::TextStyleAlignment::Trailing,
            },
            max_lines: style.line_limit,
        }
    }

    fn color(color: Color) -> zpd_winui3::Color {
        zpd_winui3::Color {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha: color.alpha,
        }
    }

    /// Applies a canvas's redraw policy, focusability, and input reporting.
    fn apply_canvas(
        canvas: &zpd_winui3::Canvas,
        reports_input: &Cell<bool>,
        old: Option<&ViewKind>,
        new: &ViewKind,
    ) -> Result<(), AppError> {
        let settings = |kind: &ViewKind| match kind {
            ViewKind::Canvas2D {
                redraw,
                focusable,
                reports_input,
                ..
            } => Some((*redraw, *focusable, *reports_input)),
            #[cfg(feature = "wgpu")]
            ViewKind::WgpuCanvas {
                redraw,
                focusable,
                reports_input,
                ..
            } => Some((*redraw, *focusable, *reports_input)),
            _ => None,
        };
        let Some((redraw, focusable, reports)) = settings(new) else {
            return Ok(());
        };
        reports_input.set(reports);
        let old = old.and_then(settings);
        if old.is_none_or(|(old, _, _)| old != redraw) {
            match redraw {
                RedrawPolicy::OnDemand => canvas.set_redraw(false, None)?,
                RedrawPolicy::Continuous { preferred_fps } => {
                    canvas.set_redraw(true, preferred_fps.map(f64::from))?;
                }
            }
        }
        if old.is_none_or(|(_, old, _)| old != focusable) {
            canvas.set_focusable(focusable)?;
        }
        Ok(())
    }

    /// Applies `new`, skipping properties equal in `old`.
    ///
    /// User-editable values are compared with the native state instead, so a
    /// value the user just entered is not written back.
    pub(super) fn apply(&self, old: Option<&ViewKind>, new: &ViewKind) -> Result<(), AppError> {
        let control = match &self.view {
            NativeView::Control(control) => control,
            NativeView::Canvas {
                canvas,
                reports_input,
                ..
            } => return Self::apply_canvas(canvas, reports_input, old, new),
        };
        match new {
            ViewKind::Container | ViewKind::Separator => {}
            ViewKind::Canvas2D { .. } => {}
            #[cfg(feature = "wgpu")]
            ViewKind::WgpuCanvas { .. } => {}
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
                // The title color and the destructive red share the foreground; the color wins.
                let recolor = old.is_none_or(|old| {
                    old.title_color != button.title_color || old.destructive != button.destructive
                });
                match button.title_color {
                    Some(color) if recolor => control.set_foreground(Some(Self::color(color)))?,
                    None if old.is_some_and(|old| old.title_color.is_some()) => {
                        control.set_foreground(None)?;
                        control.set_bool(BoolProperty::Destructive, button.destructive)?;
                    }
                    _ => {}
                }
                if changed(|button| button.shows_menu) {
                    control.set_bool(BoolProperty::ShowsMenu, button.shows_menu)?;
                }
                if changed(|button| button.keeps_state) {
                    control.set_bool(BoolProperty::KeepsState, button.keeps_state)?;
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
                selectable,
                bordered,
                draws_background,
                multiline,
                style,
            } => {
                let old = match old {
                    Some(ViewKind::TextField {
                        value,
                        placeholder,
                        selectable,
                        bordered,
                        draws_background,
                        multiline,
                        style,
                        ..
                    }) => Some((
                        value,
                        placeholder,
                        selectable,
                        bordered,
                        draws_background,
                        multiline,
                        style,
                    )),
                    _ => None,
                };
                // Typed text stays until the value changes, so unbound fields keep edits.
                if old.is_none_or(|old| old.0 != value) {
                    control.set_text(TextProperty::Text, value)?;
                }
                if old.is_none_or(|old| old.5 != multiline) {
                    control.set_bool(BoolProperty::Multiline, *multiline)?;
                }
                if old.is_none_or(|old| old.6 != style || old.5 != multiline) {
                    control.set_text_style(&Self::text_style(style))?;
                }
                if *editable {
                    if old.is_none_or(|old| old.1 != placeholder) {
                        control.set_text(
                            TextProperty::Placeholder,
                            placeholder.as_deref().unwrap_or(""),
                        )?;
                    }
                    if old.is_none_or(|old| old.3 != bordered) {
                        control.set_bool(BoolProperty::Bordered, *bordered)?;
                    }
                    if old.is_none_or(|old| old.4 != draws_background) {
                        control.set_bool(BoolProperty::DrawsBackground, *draws_background)?;
                    }
                } else if old.is_none_or(|old| old.2 != selectable) {
                    control.set_bool(BoolProperty::Selectable, *selectable)?;
                }
            }
            ViewKind::TextEditor { text, focus } => {
                let (old_text, old_focus) = match old {
                    Some(ViewKind::TextEditor { text, focus }) => (Some(text), Some(*focus)),
                    _ => (None, None),
                };
                // Text the editor already shows is not written back, so typing keeps the caret.
                if old_text != Some(text) && Self::attributed(&control.rich_text()?) != *text {
                    control.set_rich_text(&Self::rich_text(text))?;
                }
                // Each new focus request moves keyboard focus to the editor.
                if *focus > 0 && old_focus != Some(*focus) {
                    control.focus()?;
                }
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
                let same_items = match old {
                    Some(ViewKind::PopUpButton { items: old, .. })
                    | Some(ViewKind::SegmentedControl { segments: old, .. }) => old == items,
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
            ViewKind::NumberField {
                value,
                format,
                text,
            } => {
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
                // The value comes first, so declared text shows for the new value.
                // Typed numbers stay until the value changes, so unbound fields keep edits.
                // Out-of-range input is always replaced, as it was clamped.
                let shown = control.number(NumberProperty::Value)?;
                let in_range = (format.minimum..=format.maximum).contains(&shown);
                match old {
                    Some(ViewKind::NumberField { value: old, .. }) if old == value && in_range => {}
                    Some(_) if shown == *value => {}
                    _ => control.set_number(NumberProperty::Value, *value)?,
                }
                if !matches!(old, Some(ViewKind::NumberField { text: old, .. }) if old == text) {
                    control.set_text(TextProperty::Text, text.as_deref().unwrap_or(""))?;
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
                    control.set_color(Self::color(*color))?;
                }
            }
            ViewKind::ProgressIndicator { value } => {
                if let Some(value) = value
                    && !matches!(old, Some(ViewKind::ProgressIndicator { value: Some(old) }) if old == value)
                {
                    control.set_number(NumberProperty::Value, *value)?;
                }
            }
            ViewKind::ScrollView {
                axes,
                offset,
                reveal,
            } => {
                if !matches!(old, Some(ViewKind::ScrollView { axes: old, .. }) if old == axes) {
                    control.set_bool(BoolProperty::HorizontalScroll, axes.horizontal)?;
                    self.horizontal.set(axes.horizontal);
                    control.set_bool(BoolProperty::VerticalScroll, axes.vertical)?;
                }
                // Later offsets echo the user's scrolling, so only a new view restores one.
                if old.is_none() {
                    self.restore.set(*offset);
                }
                // The target scrolls into view once layout has placed it.
                if !matches!(old, Some(ViewKind::ScrollView { reveal: old, .. }) if old == reveal) {
                    self.reveal.replace(reveal.clone());
                }
            }
            ViewKind::List { selected, .. } => {
                let old = match old {
                    Some(ViewKind::List { selected, .. }) => Some(selected),
                    _ => None,
                };
                self.columns.borrow_mut().clear();
                self.apply_selection(control, old, selected)?;
            }
            ViewKind::Table {
                columns,
                selected,
                striped,
                ..
            } => {
                let (old_selected, old_striped) = match old {
                    Some(ViewKind::Table {
                        selected, striped, ..
                    }) => (Some(selected), Some(striped)),
                    _ => (None, None),
                };
                // New columns take effect at the next layout.
                if *self.columns.borrow() != *columns {
                    self.columns.replace(columns.clone());
                    self.header_widths.borrow_mut().clear();
                }
                if old_striped != Some(striped) {
                    control.set_bool(BoolProperty::Striped, *striped)?;
                }
                self.apply_selection(control, old_selected, selected)?;
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

    /// Selects and reveals a list row whose declared selection changed.
    fn apply_selection(
        &self,
        control: &Control,
        old: Option<&Option<String>>,
        selected: &Option<String>,
    ) -> Result<(), AppError> {
        if old != Some(selected) {
            control.select_list_row(selected.as_deref())?;
            if let Some(id) = selected {
                control.reveal_list_row(id)?;
            }
        }
        Ok(())
    }

    /// Splits `width` between table columns; columns without a width share the rest.
    pub(super) fn column_widths(columns: &[TableColumn], width: f32) -> Vec<f32> {
        const MINIMUM: f32 = 40.0;
        if columns.is_empty() {
            return vec![width];
        }
        let fixed: f32 = columns
            .iter()
            .filter_map(|column| column.width)
            .map(|width| width as f32)
            .sum();
        let flexible = columns
            .iter()
            .filter(|column| column.width.is_none())
            .count();
        let share = if flexible == 0 {
            0.0
        } else {
            ((width - fixed) / flexible as f32).max(MINIMUM)
        };
        columns
            .iter()
            .map(|column| column.width.map_or(share, |width| width as f32))
            .collect()
    }

    /// Lays out list cells row by row: each fills its column, and a row is as
    /// tall as its tallest cell.
    fn layout_rows(
        &self,
        control: &Control,
        frame: LayoutRect,
        hosted: &[LayoutSnapshot<Self>],
    ) -> Result<(), AppError> {
        let columns = self.columns.borrow();
        let width = (frame.width - zpd_winui3::LIST_ROW_INSET as f32).max(0.0);
        let widths = Self::column_widths(&columns, width);
        let mut heights = Vec::with_capacity(hosted.len().div_ceil(widths.len()));
        for row in hosted.chunks(widths.len()) {
            let mut sizes = Vec::with_capacity(row.len());
            for (cell, width) in row.iter().zip(&widths) {
                let mut cell = cell.clone();
                cell.style.width = LayoutDimension::Percent(1.0);
                let size = HostLayout::place(
                    std::slice::from_ref(&cell),
                    0.0,
                    Some(*width),
                    None,
                    Size::default(),
                    None,
                )?;
                sizes.push(size.height.ceil().max(1.0));
            }
            let height = sizes.iter().copied().fold(1.0, f32::max);
            let mut x = 0.0;
            for (cell, width) in row.iter().zip(&widths) {
                cell.native.apply_frame(
                    LayoutRect {
                        x,
                        y: 0.0,
                        width: *width,
                        height,
                    },
                    None,
                )?;
                x += width;
            }
            heights.push(f64::from(height));
        }
        control.set_list_row_heights(&heights)?;
        if !columns.is_empty() && *self.header_widths.borrow() != widths {
            let titled = columns
                .iter()
                .zip(&widths)
                .map(|(column, width)| (column.title.as_str(), f64::from(*width)))
                .collect::<Vec<_>>();
            control.set_list_columns(&titled)?;
            self.header_widths.replace(widths);
        }
        Ok(())
    }
}

impl LayoutNative for NativeNode {
    type Error = AppError;

    fn apply_frame(&self, frame: LayoutRect, _parent_height: Option<f32>) -> Result<(), AppError> {
        Ok(self.view.set_frame(
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
        let Some(control) = self.control() else {
            return Ok(());
        };
        match control.kind() {
            ControlKind::ScrollViewer => {
                let (width, height) = (frame.width, frame.height);
                // Content that scrolls sideways takes its own width, at least the visible one.
                let fixed_width = (!self.horizontal.get()).then_some(width);
                let size = HostLayout::place(
                    hosted,
                    0.0,
                    fixed_width,
                    None,
                    Size::new(width, height),
                    None,
                )?;
                control.set_scroll_extent(
                    f64::from(size.width.max(width)),
                    f64::from(size.height.max(height)),
                )?;
                if let Some((x, y)) = self.restore.take() {
                    control.scroll_to(x, y)?;
                }
                // Nested views are placed with the document, so the target can be found now.
                if let Some(target) = self.reveal.take() {
                    control.reveal(
                        &target.id,
                        match target.alignment {
                            ScrollAlignment::Nearest => zpd_winui3::ScrollAlignment::Nearest,
                            ScrollAlignment::Start => zpd_winui3::ScrollAlignment::Start,
                            ScrollAlignment::Center => zpd_winui3::ScrollAlignment::Center,
                            ScrollAlignment::End => zpd_winui3::ScrollAlignment::End,
                        },
                    )?;
                }
            }
            ControlKind::ListView => self.layout_rows(control, frame, hosted)?,
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
        ("arrow.clockwise", "\u{E72C}"),
        ("bell", "\u{EA8F}"),
        ("bookmark", "\u{E8A4}"),
        ("calendar", "\u{E787}"),
        ("checkmark", "\u{E73E}"),
        ("cursorarrow.click", "\u{E7C9}"),
        ("doc", "\u{E8A5}"),
        ("envelope", "\u{E715}"),
        ("folder", "\u{E8B7}"),
        ("gearshape", "\u{E713}"),
        ("heart", "\u{EB51}"),
        ("house", "\u{E80F}"),
        ("info.circle", "\u{E946}"),
        ("list.bullet", "\u{E8FD}"),
        ("macwindow.on.rectangle", "\u{E8A7}"),
        ("magnifyingglass", "\u{E721}"),
        ("minus", "\u{E738}"),
        ("paintpalette", "\u{E790}"),
        ("pause.fill", "\u{E769}"),
        ("pencil", "\u{E70F}"),
        ("person", "\u{E77B}"),
        ("photo", "\u{E91B}"),
        ("pin", "\u{E718}"),
        ("play.fill", "\u{E768}"),
        ("plus", "\u{E710}"),
        ("questionmark.circle", "\u{E897}"),
        ("slider.horizontal.3", "\u{E9E9}"),
        ("square.and.arrow.down", "\u{E74E}"),
        ("square.and.arrow.up", "\u{E72D}"),
        ("square.and.pencil", "\u{E70F}"),
        ("square.split.2x1", "\u{E8A9}"),
        ("star", "\u{E734}"),
        ("tag", "\u{E8EC}"),
        ("trash", "\u{E74D}"),
        ("tray.full", "\u{E8F1}"),
        ("xmark", "\u{E711}"),
    ];

    /// Returns the glyph for an SF Symbol name; unknown names show no icon.
    pub(super) fn of(name: &str) -> Option<&'static str> {
        Self::TABLE
            .binary_search_by(|(candidate, _)| (*candidate).cmp(name))
            .ok()
            .map(|index| Self::TABLE[index].1)
    }
}
