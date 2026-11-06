//! Native controls addressed through typed properties and one event channel.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use super::{
    AsElement, ElementRef, Error, Font, MenuFlyoutItemBase, NativeMenuItem, OwnedElement, Result,
    abort_on_panic, check, clone_callback, ffi, release_callback,
};

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    /// Positions children at explicit frames and reports its size.
    Canvas = 0,
    TextBlock = 1,
    TextBox = 2,
    Button = 3,
    ToggleButton = 4,
    CheckBox = 5,
    RadioButton = 6,
    ToggleSwitch = 7,
    Slider = 8,
    ComboBox = 9,
    EditableComboBox = 10,
    SelectorBar = 11,
    NumberBox = 12,
    AutoSuggestBox = 13,
    /// A button that opens a color picker flyout.
    ColorButton = 14,
    ColorPicker = 15,
    /// Scrolls a Canvas document.
    ScrollViewer = 16,
    TabView = 17,
    NavigationView = 18,
    /// Edits rich text; see [`Control::set_rich_text`].
    RichEditBox = 19,
    /// Selectable rows with optional section and column headers.
    ListView = 20,
    ProgressRing = 21,
    /// Shows [`NumberProperty::Value`] in `0.0..=1.0`.
    ProgressBar = 22,
    /// A one-pixel line along the longer side of its frame.
    Separator = 23,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextProperty {
    /// Content, value, or label text.
    Text = 0,
    Placeholder = 1,
    /// A Segoe Fluent Icons glyph shown before button content.
    Glyph = 2,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberProperty {
    Value = 0,
    Minimum = 1,
    Maximum = 2,
    Step = 3,
    /// Slider tick marks; zero hides them.
    TickCount = 4,
    /// Negative values clear the selection.
    SelectedIndex = 5,
    FractionDigits = 6,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolProperty {
    Checked = 0,
    Enabled = 1,
    ReadOnly = 2,
    Multiline = 3,
    Borderless = 4,
    Destructive = 5,
    SnapToTicks = 6,
    HorizontalScroll = 7,
    VerticalScroll = 8,
    /// Draws a round help button.
    Help = 9,
    /// Accent style; the button answers Enter.
    DefaultAction = 10,
    /// The button answers Escape.
    CancelAction = 11,
    Alpha = 12,
    Bordered = 13,
    DrawsBackground = 14,
    Selectable = 15,
    /// Alternates list row backgrounds.
    Striped = 16,
    /// A click shows the context menu below the button instead of reporting it.
    ShowsMenu = 17,
    /// Clicks report the toggled state but keep the declared one.
    KeepsState = 18,
}

/// An sRGB color with components in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Color {
    pub(crate) fn native(self) -> ffi::Color {
        ffi::Color {
            red: self.red,
            green: self.green,
            blue: self.blue,
            alpha: self.alpha,
        }
    }

    fn from_native(color: ffi::Color) -> Self {
        Self {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha: color.alpha,
        }
    }
}

/// A change made by the user; programmatic updates are not reported.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlEvent {
    Clicked,
    TextChanged(String),
    StateChanged(bool),
    ValueChanged(f64),
    SelectionChanged(usize),
    ColorChanged(Color),
    Scrolled {
        x: f64,
        y: f64,
    },
    /// `inset` is the height of a canvas's top part under the title bar.
    SizeChanged {
        width: f64,
        height: f64,
        inset: f64,
    },
    NavigationSelected(String),
    SearchChanged(String),
    RichTextChanged(RichText),
    /// The selected row id, or `None` when the selection was cleared.
    ListSelectionChanged(Option<String>),
    /// An item of a navigation item's context menu was chosen.
    MenuSelected(String),
    /// Editing ended with Enter or a focus change.
    TextCommitted(String),
}

impl ControlEvent {
    /// Converts a native event; `None` marks an unknown kind.
    ///
    /// # Safety
    /// `event.text` must point to valid UTF-8 for the duration of the call.
    unsafe fn from_native(event: &ffi::Event) -> Option<Self> {
        // SAFETY: The caller guarantees the text buffer for this synchronous call.
        let text = || unsafe { super::string_from_ref(event.text) };
        Some(match event.kind {
            0 => Self::Clicked,
            1 => Self::TextChanged(text()),
            2 => Self::StateChanged(event.flag),
            3 => Self::ValueChanged(event.x),
            4 => Self::SelectionChanged(usize::try_from(event.index).ok()?),
            5 => Self::ColorChanged(Color::from_native(event.color)),
            6 => Self::Scrolled {
                x: event.x,
                y: event.y,
            },
            7 => Self::SizeChanged {
                width: event.x,
                height: event.y,
                inset: event.inset,
            },
            8 => Self::NavigationSelected(text()),
            9 => Self::SearchChanged(text()),
            // SAFETY: Native code passes a live document for this kind.
            10 => Self::RichTextChanged(unsafe { RichText::from_native(event.rich.as_ref()?) }),
            11 => Self::ListSelectionChanged(event.flag.then(text)),
            14 => Self::MenuSelected(text()),
            15 => Self::TextCommitted(text()),
            _ => return None,
        })
    }
}

/// An item in a [`ControlKind::NavigationView`] pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationItem {
    /// A nonselectable section title.
    Header(String),
    Item {
        id: String,
        title: String,
        /// A Segoe Fluent Icons glyph.
        glyph: Option<String>,
        /// Trailing text such as a count.
        badge: Option<String>,
        /// A context menu whose choices report [`ControlEvent::MenuSelected`].
        menu: Vec<MenuFlyoutItemBase>,
    },
}

/// How a TextBlock or TextBox draws its text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle<'a> {
    /// `None` keeps the control's font.
    pub font: Option<Font<'a>>,
    pub color: TextColor,
    pub alignment: TextStyleAlignment,
    /// Lines shown before the last one is truncated; `None` keeps the control's wrapping.
    pub max_lines: Option<usize>,
}

/// A theme text color, or a fixed one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextColor {
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Accent,
    Custom(Color),
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextStyleAlignment {
    #[default]
    Leading = 0,
    Center = 1,
    Trailing = 2,
}

/// Character formatting of a [`RichRun`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextTraits {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub monospace: bool,
}

impl TextTraits {
    fn bits(self) -> u32 {
        u32::from(self.bold)
            | u32::from(self.italic) << 1
            | u32::from(self.underline) << 2
            | u32::from(self.strikethrough) << 3
            | u32::from(self.monospace) << 4
    }

    fn from_bits(bits: u32) -> Self {
        Self {
            bold: bits & 1 != 0,
            italic: bits & 2 != 0,
            underline: bits & 4 != 0,
            strikethrough: bits & 8 != 0,
            monospace: bits & 16 != 0,
        }
    }
}

/// The role of a rich text paragraph.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParagraphStyle {
    Title = 0,
    Heading = 1,
    Subheading = 2,
    #[default]
    Body = 3,
    Monospaced = 4,
    Bulleted = 5,
    Dashed = 6,
    Numbered = 7,
    Checklist = 8,
    ChecklistDone = 9,
}

impl ParagraphStyle {
    fn from_native(value: i32) -> Self {
        match value {
            0 => Self::Title,
            1 => Self::Heading,
            2 => Self::Subheading,
            4 => Self::Monospaced,
            5 => Self::Bulleted,
            6 => Self::Dashed,
            7 => Self::Numbered,
            8 => Self::Checklist,
            9 => Self::ChecklistDone,
            _ => Self::Body,
        }
    }
}

/// Text with one set of traits; paragraphs end with `'\n'`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RichRun {
    pub text: String,
    pub traits: TextTraits,
    pub link: Option<String>,
}

/// A RichEditBox document: runs that cover the text, and one style per paragraph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RichText {
    pub runs: Vec<RichRun>,
    pub paragraphs: Vec<ParagraphStyle>,
}

impl RichText {
    /// Copies a native document.
    ///
    /// # Safety
    /// Every pointer in `text` must be valid for the duration of the call.
    unsafe fn from_native(text: &ffi::RichText) -> Self {
        let slice = |data, length| {
            if length == 0 {
                &[][..]
            } else {
                // SAFETY: The caller guarantees `length` readable elements.
                unsafe { std::slice::from_raw_parts(data, length) }
            }
        };
        let runs = slice(text.runs, text.runs_length)
            .iter()
            .map(|run: &ffi::RichRun| RichRun {
                // SAFETY: The caller guarantees the run strings.
                text: unsafe { super::string_from_ref(run.text) },
                traits: TextTraits::from_bits(run.traits),
                // SAFETY: The caller guarantees the run strings.
                link: Some(unsafe { super::string_from_ref(run.link) })
                    .filter(|link| !link.is_empty()),
            })
            .collect();
        let paragraphs = if text.paragraphs_length == 0 {
            Vec::new()
        } else {
            // SAFETY: The caller guarantees `paragraphs_length` readable styles.
            unsafe { std::slice::from_raw_parts(text.paragraphs, text.paragraphs_length) }
                .iter()
                .copied()
                .map(ParagraphStyle::from_native)
                .collect()
        };
        Self { runs, paragraphs }
    }
}

/// How much narrower list rows are than their list; cells and column headers start after half of it.
pub const LIST_ROW_INSET: f64 = 24.0;

/// An entry of a [`ControlKind::ListView`].
pub enum ListEntry<'a> {
    /// A section title that cannot be selected.
    Header(&'a str),
    /// A row whose cells are laid out side by side.
    Row {
        id: &'a str,
        cells: Vec<ElementRef<'a>>,
    },
}

/// Where a revealed element sits in a scroll viewer's visible area.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAlignment {
    #[default]
    Nearest = 0,
    Start = 1,
    Center = 2,
    End = 3,
}

/// A WinUI control created for a [`ControlKind`].
pub struct Control {
    kind: ControlKind,
    element: OwnedElement,
}

impl AsElement for Control {
    fn as_element(&self) -> ElementRef<'_> {
        self.element.as_element()
    }
}

impl Control {
    pub fn new(kind: ControlKind) -> Result<Self> {
        // SAFETY: Native creation runs on the UI thread and returns an owned wrapper.
        let raw = unsafe { ffi::zpd_winui3_control_create(kind as i32) };
        Ok(Self {
            kind,
            // SAFETY: A non-null result is a newly owned control wrapper.
            element: unsafe { OwnedElement::from_raw(raw)? },
        })
    }

    #[must_use]
    pub fn kind(&self) -> ControlKind {
        self.kind
    }

    fn raw(&self) -> *mut ffi::Element {
        self.element.raw.as_ptr()
    }

    pub fn set_text(&self, property: TextProperty, value: &str) -> Result<()> {
        // SAFETY: The control is live and the string is borrowed for this call.
        check(unsafe {
            ffi::zpd_winui3_control_set_text(
                self.raw(),
                property as i32,
                ffi::StringRef::new(value),
            )
        })
    }

    pub fn text(&self, property: TextProperty) -> Result<String> {
        unsafe extern "C" fn receive(data: *const c_void, value: ffi::StringRef) {
            // SAFETY: `data` is the String below and `value` is valid for this call.
            unsafe { *data.cast_mut().cast::<String>() = super::string_from_ref(value) };
        }
        let mut result = String::new();
        // SAFETY: `receive` writes into `result` synchronously during this call.
        check(unsafe {
            ffi::zpd_winui3_control_get_text(
                self.raw(),
                property as i32,
                std::ptr::from_mut(&mut result).cast(),
                receive,
            )
        })?;
        Ok(result)
    }

    pub fn set_number(&self, property: NumberProperty, value: f64) -> Result<()> {
        // SAFETY: The control is live on the UI thread.
        check(unsafe { ffi::zpd_winui3_control_set_number(self.raw(), property as i32, value) })
    }

    /// Returns the value, or NaN when the control has no such number.
    pub fn number(&self, property: NumberProperty) -> Result<f64> {
        let mut value = f64::NAN;
        // SAFETY: The control is live and `value` outlives the call.
        check(unsafe {
            ffi::zpd_winui3_control_get_number(self.raw(), property as i32, &raw mut value)
        })?;
        Ok(value)
    }

    pub fn set_bool(&self, property: BoolProperty, value: bool) -> Result<()> {
        // SAFETY: The control is live on the UI thread.
        check(unsafe { ffi::zpd_winui3_control_set_bool(self.raw(), property as i32, value) })
    }

    pub fn bool(&self, property: BoolProperty) -> Result<bool> {
        let mut value = false;
        // SAFETY: The control is live and `value` outlives the call.
        check(unsafe {
            ffi::zpd_winui3_control_get_bool(self.raw(), property as i32, &raw mut value)
        })?;
        Ok(value)
    }

    pub fn set_color(&self, color: Color) -> Result<()> {
        // SAFETY: The control is live on the UI thread.
        check(unsafe { ffi::zpd_winui3_control_set_color(self.raw(), color.native()) })
    }

    /// Colors a control's text, or restores the theme's color for `None`.
    pub fn set_foreground(&self, color: Option<Color>) -> Result<()> {
        let color = color.map(Color::native);
        let value = color.as_ref().map_or(std::ptr::null(), std::ptr::from_ref);
        // SAFETY: The control is live on the UI thread; `value` is null or outlives the call.
        check(unsafe { ffi::zpd_winui3_control_set_foreground(self.raw(), value) })
    }

    pub fn color(&self) -> Result<Color> {
        let mut value = ffi::Color::default();
        // SAFETY: The control is live and `value` outlives the call.
        check(unsafe { ffi::zpd_winui3_control_get_color(self.raw(), &raw mut value) })?;
        Ok(Color::from_native(value))
    }

    /// Sets choices, segments, or tab titles.
    pub fn set_items<S: AsRef<str>>(&self, items: &[S]) -> Result<()> {
        let raw: Vec<_> = items
            .iter()
            .map(|item| ffi::StringRef::new(item.as_ref()))
            .collect();
        // SAFETY: The strings are borrowed for this synchronous call.
        check(unsafe { ffi::zpd_winui3_control_set_items(self.raw(), raw.as_ptr(), raw.len()) })
    }

    /// Sets Canvas children, scroll content, tab pages, or navigation content.
    pub fn set_children(&self, children: &[ElementRef<'_>]) -> Result<()> {
        let raw: Vec<_> = children.iter().map(ElementRef::as_ptr).collect();
        // SAFETY: Every child is live and native code retains the XAML elements.
        check(unsafe { ffi::zpd_winui3_control_set_children(self.raw(), raw.as_ptr(), raw.len()) })
    }

    /// Installs the handler for user changes, replacing any earlier one.
    pub fn set_event_handler<F>(&self, callback: F) -> Result<()>
    where
        F: FnMut(ControlEvent) + 'static,
    {
        unsafe extern "C" fn invoke<F: FnMut(ControlEvent)>(
            data: *const c_void,
            event: *const ffi::Event,
        ) {
            abort_on_panic(|| {
                // SAFETY: Native code passes a live event for this synchronous call.
                let Some(event) = (unsafe { ControlEvent::from_native(&*event) }) else {
                    return;
                };
                // SAFETY: This callback was installed with an Rc-backed state of type F.
                let callback = unsafe { clone_callback::<F>(data) };
                // A nested native event while the handler runs is dropped.
                if let Ok(mut callback) = callback.try_borrow_mut() {
                    callback(event);
                }
            });
        }

        let callback = Rc::into_raw(Rc::new(RefCell::new(callback)));
        // SAFETY: Native code takes ownership of one Rc strong reference.
        check(unsafe {
            ffi::zpd_winui3_control_set_event_handler(
                self.raw(),
                callback.cast(),
                invoke::<F>,
                release_callback::<F>,
            )
        })
    }

    /// Sizes the scrollable document of a [`ControlKind::ScrollViewer`].
    pub fn set_scroll_extent(&self, width: f64, height: f64) -> Result<()> {
        self.expect(ControlKind::ScrollViewer)?;
        // SAFETY: The control is a live ScrollViewer wrapper.
        check(unsafe { ffi::zpd_winui3_scroll_viewer_set_extent(self.raw(), width, height) })
    }

    pub fn scroll_to(&self, x: f64, y: f64) -> Result<()> {
        self.expect(ControlKind::ScrollViewer)?;
        // SAFETY: The control is a live ScrollViewer wrapper.
        check(unsafe { ffi::zpd_winui3_scroll_viewer_scroll_to(self.raw(), x, y) })
    }

    pub fn set_navigation_items(&self, items: &[NavigationItem]) -> Result<()> {
        self.expect(ControlKind::NavigationView)?;
        // Menu storage stays alive for the call.
        let menus: Vec<Vec<NativeMenuItem>> = items
            .iter()
            .map(|item| match item {
                NavigationItem::Item { menu, .. } => menu.iter().map(NativeMenuItem::new).collect(),
                NavigationItem::Header(_) => Vec::new(),
            })
            .collect();
        let menu_raw: Vec<Vec<ffi::MenuFlyoutItem>> = menus
            .iter()
            .map(|menu| menu.iter().map(|item| item.raw).collect())
            .collect();
        let raw: Vec<_> = items
            .iter()
            .zip(&menu_raw)
            .map(|(item, menu)| match item {
                NavigationItem::Header(title) => ffi::NavigationItem {
                    header: true,
                    id: ffi::StringRef::new(""),
                    title: ffi::StringRef::new(title),
                    glyph: ffi::StringRef::new(""),
                    badge: ffi::StringRef::new(""),
                    menu: std::ptr::null(),
                    menu_length: 0,
                },
                NavigationItem::Item {
                    id,
                    title,
                    glyph,
                    badge,
                    ..
                } => ffi::NavigationItem {
                    header: false,
                    id: ffi::StringRef::new(id),
                    title: ffi::StringRef::new(title),
                    glyph: ffi::StringRef::new(glyph.as_deref().unwrap_or("")),
                    badge: ffi::StringRef::new(badge.as_deref().unwrap_or("")),
                    menu: menu.as_ptr(),
                    menu_length: menu.len(),
                },
            })
            .collect();
        // SAFETY: The item strings are borrowed for this synchronous call.
        check(unsafe { ffi::zpd_winui3_navigation_set_items(self.raw(), raw.as_ptr(), raw.len()) })
    }

    pub fn select_navigation_item(&self, id: Option<&str>) -> Result<()> {
        self.expect(ControlKind::NavigationView)?;
        // SAFETY: The control is a live NavigationView and the id is borrowed.
        check(unsafe {
            ffi::zpd_winui3_navigation_select(self.raw(), ffi::StringRef::new(id.unwrap_or("")))
        })
    }

    /// Shows a search box with `(value, placeholder)`, or hides it for `None`.
    pub fn set_navigation_search(&self, search: Option<(&str, &str)>) -> Result<()> {
        self.expect(ControlKind::NavigationView)?;
        let (value, placeholder) = search.unwrap_or_default();
        // SAFETY: The control is a live NavigationView and the strings are borrowed.
        check(unsafe {
            ffi::zpd_winui3_navigation_set_search(
                self.raw(),
                search.is_some(),
                ffi::StringRef::new(value),
                ffi::StringRef::new(placeholder),
            )
        })
    }

    /// Styles the text of a TextBlock or TextBox.
    pub fn set_text_style(&self, style: &TextStyle<'_>) -> Result<()> {
        let (color, custom) = match style.color {
            TextColor::Primary => (0, Color::default()),
            TextColor::Secondary => (1, Color::default()),
            TextColor::Tertiary => (2, Color::default()),
            TextColor::Accent => (3, Color::default()),
            TextColor::Custom(color) => (4, color),
        };
        let font = style.font.unwrap_or(Font {
            family: None,
            size: 0.0,
            weight: 0,
            italic: false,
        });
        let native = ffi::TextStyle {
            font: ffi::Font {
                family: ffi::StringRef::new(font.family.unwrap_or("")),
                size: font.size,
                weight: i32::from(font.weight),
                italic: font.italic,
            },
            color,
            custom: custom.native(),
            alignment: style.alignment as i32,
            max_lines: style
                .max_lines
                .map_or(0, |lines| i32::try_from(lines).unwrap_or(i32::MAX)),
        };
        // SAFETY: The control is live and the style's strings outlive the call.
        check(unsafe { ffi::zpd_winui3_control_set_text_style(self.raw(), &raw const native) })
    }

    /// Replaces the document of a [`ControlKind::RichEditBox`] without reporting a change.
    pub fn set_rich_text(&self, text: &RichText) -> Result<()> {
        self.expect(ControlKind::RichEditBox)?;
        let runs: Vec<_> = text
            .runs
            .iter()
            .map(|run| ffi::RichRun {
                text: ffi::StringRef::new(&run.text),
                traits: run.traits.bits(),
                link: ffi::StringRef::new(run.link.as_deref().unwrap_or("")),
            })
            .collect();
        let paragraphs: Vec<i32> = text.paragraphs.iter().map(|style| *style as i32).collect();
        let native = ffi::RichText {
            runs: runs.as_ptr(),
            runs_length: runs.len(),
            paragraphs: paragraphs.as_ptr(),
            paragraphs_length: paragraphs.len(),
        };
        // SAFETY: The control is a live RichEditBox and the document storage outlives the call.
        check(unsafe { ffi::zpd_winui3_rich_text_set(self.raw(), &raw const native) })
    }

    /// Returns the document of a [`ControlKind::RichEditBox`].
    pub fn rich_text(&self) -> Result<RichText> {
        unsafe extern "C" fn receive(data: *const c_void, text: *const ffi::RichText) {
            // SAFETY: `data` is the RichText below and `text` is live for this call.
            unsafe { *data.cast_mut().cast::<RichText>() = RichText::from_native(&*text) };
        }
        self.expect(ControlKind::RichEditBox)?;
        let mut result = RichText::default();
        // SAFETY: `receive` writes into `result` synchronously during this call.
        check(unsafe {
            ffi::zpd_winui3_rich_text_get(
                self.raw(),
                std::ptr::from_mut(&mut result).cast(),
                receive,
            )
        })?;
        Ok(result)
    }

    /// Replaces the headers and rows of a [`ControlKind::ListView`].
    pub fn set_list_entries(&self, entries: &[ListEntry<'_>]) -> Result<()> {
        self.expect(ControlKind::ListView)?;
        let cells: Vec<Vec<*const ffi::Element>> = entries
            .iter()
            .map(|entry| match entry {
                ListEntry::Row { cells, .. } => cells.iter().map(ElementRef::as_ptr).collect(),
                ListEntry::Header(_) => Vec::new(),
            })
            .collect();
        let raw: Vec<_> = entries
            .iter()
            .zip(&cells)
            .map(|(entry, cells)| match entry {
                ListEntry::Header(title) => ffi::ListEntry {
                    header: true,
                    id: ffi::StringRef::new(""),
                    title: ffi::StringRef::new(title),
                    cells: std::ptr::null(),
                    cells_length: 0,
                },
                ListEntry::Row { id, .. } => ffi::ListEntry {
                    header: false,
                    id: ffi::StringRef::new(id),
                    title: ffi::StringRef::new(""),
                    cells: cells.as_ptr(),
                    cells_length: cells.len(),
                },
            })
            .collect();
        // SAFETY: Every element is live and the entry storage outlives the call.
        check(unsafe { ffi::zpd_winui3_list_set_entries(self.raw(), raw.as_ptr(), raw.len()) })
    }

    /// Shows column headers `(title, width)`; an empty slice hides them.
    pub fn set_list_columns(&self, columns: &[(&str, f64)]) -> Result<()> {
        self.expect(ControlKind::ListView)?;
        let titles: Vec<_> = columns
            .iter()
            .map(|(title, _)| ffi::StringRef::new(title))
            .collect();
        let widths: Vec<_> = columns.iter().map(|(_, width)| *width).collect();
        // SAFETY: The strings and widths are borrowed for this synchronous call.
        check(unsafe {
            ffi::zpd_winui3_list_set_columns(
                self.raw(),
                titles.as_ptr(),
                widths.as_ptr(),
                columns.len(),
            )
        })
    }

    /// Sizes the list's rows in order, skipping headers.
    pub fn set_list_row_heights(&self, heights: &[f64]) -> Result<()> {
        self.expect(ControlKind::ListView)?;
        // SAFETY: The heights are borrowed for this synchronous call.
        check(unsafe {
            ffi::zpd_winui3_list_set_row_heights(self.raw(), heights.as_ptr(), heights.len())
        })
    }

    /// Selects the row with `id` without reporting it; `None` clears the selection.
    pub fn select_list_row(&self, id: Option<&str>) -> Result<()> {
        self.expect(ControlKind::ListView)?;
        // SAFETY: The control is a live ListView and the id is borrowed.
        check(unsafe {
            ffi::zpd_winui3_list_select(self.raw(), ffi::StringRef::new(id.unwrap_or("")))
        })
    }

    pub fn reveal_list_row(&self, id: &str) -> Result<()> {
        self.expect(ControlKind::ListView)?;
        // SAFETY: The control is a live ListView and the id is borrowed.
        check(unsafe { ffi::zpd_winui3_list_reveal(self.raw(), ffi::StringRef::new(id)) })
    }

    /// Scrolls the descendant whose identifier is `id` into view.
    pub fn reveal(&self, id: &str, alignment: ScrollAlignment) -> Result<()> {
        self.expect(ControlKind::ScrollViewer)?;
        // SAFETY: The control is a live ScrollViewer and the id is borrowed.
        check(unsafe {
            ffi::zpd_winui3_scroll_viewer_reveal(
                self.raw(),
                ffi::StringRef::new(id),
                alignment as i32,
            )
        })
    }

    fn expect(&self, kind: ControlKind) -> Result<()> {
        if self.kind == kind {
            Ok(())
        } else {
            Err(Error(E_INVALIDARG))
        }
    }
}

/// `E_INVALIDARG`, reported when an operation does not apply to a control kind.
#[allow(clippy::cast_possible_wrap)]
const E_INVALIDARG: i32 = 0x8007_0057_u32 as i32;

#[cfg(test)]
mod tests {
    use super::*;

    fn native(kind: i32, text: &str) -> ffi::Event {
        ffi::Event {
            kind,
            x: 1.5,
            y: 2.5,
            index: 3,
            flag: true,
            text: ffi::StringRef::new(text),
            color: ffi::Color {
                red: 0.25,
                green: 0.5,
                blue: 0.75,
                alpha: 1.0,
            },
            inset: 4.0,
            rich: std::ptr::null(),
        }
    }

    #[test]
    fn native_events_map_to_their_payloads() {
        // Verifies each native event kind reads the fields the C++ side fills for it.
        // SAFETY: Every test event points at a live string literal.
        let convert = |kind, text| unsafe { ControlEvent::from_native(&native(kind, text)) };
        assert_eq!(convert(0, ""), Some(ControlEvent::Clicked));
        assert_eq!(
            convert(1, "hello"),
            Some(ControlEvent::TextChanged("hello".into()))
        );
        assert_eq!(convert(2, ""), Some(ControlEvent::StateChanged(true)));
        assert_eq!(convert(3, ""), Some(ControlEvent::ValueChanged(1.5)));
        assert_eq!(convert(4, ""), Some(ControlEvent::SelectionChanged(3)));
        assert_eq!(
            convert(6, ""),
            Some(ControlEvent::Scrolled { x: 1.5, y: 2.5 })
        );
        assert_eq!(
            convert(7, ""),
            Some(ControlEvent::SizeChanged {
                width: 1.5,
                height: 2.5,
                inset: 4.0,
            })
        );
        assert_eq!(
            convert(8, "home"),
            Some(ControlEvent::NavigationSelected("home".into()))
        );
        assert_eq!(
            convert(5, ""),
            Some(ControlEvent::ColorChanged(Color {
                red: 0.25,
                green: 0.5,
                blue: 0.75,
                alpha: 1.0
            }))
        );
    }

    #[test]
    fn unknown_or_negative_selections_are_dropped() {
        // Verifies malformed native events never reach the Rust handler.
        let mut event = native(4, "");
        event.index = -1;
        // SAFETY: The event points at a live string literal.
        assert_eq!(unsafe { ControlEvent::from_native(&event) }, None);
        // SAFETY: The event points at a live string literal.
        assert_eq!(unsafe { ControlEvent::from_native(&native(99, "")) }, None);
    }

    #[test]
    fn list_selection_and_commits_carry_their_text() {
        // Verifies the list selection reads `flag` as "has a selection" and commits keep their text.
        let mut cleared = native(11, "row");
        cleared.flag = false;
        // SAFETY: The events point at live string literals.
        unsafe {
            assert_eq!(
                ControlEvent::from_native(&native(11, "row")),
                Some(ControlEvent::ListSelectionChanged(Some("row".into())))
            );
            assert_eq!(
                ControlEvent::from_native(&cleared),
                Some(ControlEvent::ListSelectionChanged(None))
            );
            assert_eq!(
                ControlEvent::from_native(&native(15, "done")),
                Some(ControlEvent::TextCommitted("done".into()))
            );
        }
    }

    #[test]
    fn text_traits_round_trip_through_bits() {
        // Verifies the trait bits match the C header and decode to the same traits.
        let traits = TextTraits {
            bold: true,
            strikethrough: true,
            ..TextTraits::default()
        };
        assert_eq!(traits.bits(), 0b1001);
        assert_eq!(TextTraits::from_bits(traits.bits()), traits);
    }
}
