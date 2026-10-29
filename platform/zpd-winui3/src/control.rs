//! Native controls addressed through typed properties and one event channel.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use super::{
    AsElement, ElementRef, Error, OwnedElement, Result, abort_on_panic, check, clone_callback, ffi,
    release_callback,
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
    fn native(self) -> ffi::Color {
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
    Scrolled { x: f64, y: f64 },
    SizeChanged { width: f64, height: f64 },
    NavigationSelected(String),
    SearchChanged(String),
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
            },
            8 => Self::NavigationSelected(text()),
            9 => Self::SearchChanged(text()),
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
    },
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
        let raw: Vec<_> = items
            .iter()
            .map(|item| match item {
                NavigationItem::Header(title) => ffi::NavigationItem {
                    header: true,
                    id: ffi::StringRef::new(""),
                    title: ffi::StringRef::new(title),
                    glyph: ffi::StringRef::new(""),
                },
                NavigationItem::Item { id, title, glyph } => ffi::NavigationItem {
                    header: false,
                    id: ffi::StringRef::new(id),
                    title: ffi::StringRef::new(title),
                    glyph: ffi::StringRef::new(glyph.as_deref().unwrap_or("")),
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
                height: 2.5
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
}
