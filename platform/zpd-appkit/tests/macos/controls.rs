// Runs native control, menu, and dialog contracts on AppKit's main thread.
use std::cell::RefCell;
use std::rc::Rc;

use zpd_appkit::actor::WindowEventKind;
use zpd_appkit::geometry::Rect;
use zpd_appkit::graphics::Font;
use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AlertButton, AlertButtonRole, AlertConfig, AlertSheet, AsView, Button, ButtonType, Color,
    DatePicker, DatePickerStyle, FieldAccessory,
    ColorWell, ListEntry, ListStyle, ListView, CommandItem, CommandMenu, CommandRole, CommandSet, ContextMenu, MenuRole,
    NumberField, NumberFormat, Pasteboard, PopUpButton, Popover, ScrollView, SegmentedControl, Sidebar,
    SidebarItem, SidebarSearch, SidebarSection, Slider, Switch, SystemColor, TabView, TextAlignment, TextColor,
    RichParagraph, RichRun, RichStyle, TableColumn, TextEditor, TextField, Toolbar, ToolbarItem, View,
    WindowStyle,
};
use zpd_objc::Id;

struct Checks;

/// An `NSRect` passed to and returned from AppKit.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct NsRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Checks {
    /// Sends a control's action to its target, as a user interaction would.
    fn send_action(control: Id) {
        // SAFETY: The control is live and has a target/action installed.
        unsafe {
            let action =
                zpd_objc::msg_send!(control, zpd_objc::sel!("action"), () => zpd_objc::Sel);
            let target = zpd_objc::msg_send!(control, zpd_objc::sel!("target"), () => Id);
            zpd_objc::msg_send!(control, zpd_objc::sel!("sendAction:to:"), ((action): zpd_objc::Sel, (target): Id) => bool);
        }
    }

    /// Resolves a view's native object through a temporary accessibility identifier.
    /// Returns the foreground color of a button title's last character, if the title has one.
    fn title_color(button: Id) -> Option<Color> {
        // SAFETY: The button is live; the attributed title and its attributes are autoreleased.
        unsafe {
            let title = zpd_objc::msg_send!(button, zpd_objc::sel!("attributedTitle"), () => Id);
            let length = zpd_objc::msg_send!(title, zpd_objc::sel!("length"), () => u64);
            let key = zpd_objc::msg_send!(zpd_objc::class!("NSString"), zpd_objc::sel!("stringWithUTF8String:"), ((c"NSColor".as_ptr()): *const std::ffi::c_char) => Id);
            let color = zpd_objc::msg_send!(title, zpd_objc::sel!("attribute:atIndex:effectiveRange:"), ((key): Id, (length - 1): u64, (std::ptr::null_mut::<u8>()): *mut u8) => Id);
            if color.is_null() {
                return None;
            }
            let space = zpd_objc::msg_send!(zpd_objc::class!("NSColorSpace"), zpd_objc::sel!("sRGBColorSpace"), () => Id);
            let color = zpd_objc::msg_send!(color, zpd_objc::sel!("colorUsingColorSpace:"), ((space): Id) => Id);
            let mut value = Color::new(0.0, 0.0, 0.0, 0.0);
            zpd_objc::msg_send!(color, zpd_objc::sel!("getRed:green:blue:alpha:"), ((&mut value.red): *mut f64, (&mut value.green): *mut f64, (&mut value.blue): *mut f64, (&mut value.alpha): *mut f64) => ());
            Some(value)
        }
    }

    fn native(view: &impl AsView) -> Id {
        const IDENTIFIER: &str = "zpd-controls-check";
        view.as_view().set_identifier(Some(IDENTIFIER)).unwrap();
        // SAFETY: Windows and their content views are live on the main thread.
        let native = unsafe {
            Self::windows()
                .into_iter()
                .map(|window| {
                    let content =
                        zpd_objc::msg_send!(window, zpd_objc::sel!("contentView"), () => Id);
                    Self::find(content, IDENTIFIER)
                })
                .find(|native| !native.is_null())
                .unwrap_or(zpd_objc::NIL)
        };
        view.as_view().set_identifier(None).unwrap();
        assert!(!native.is_null(), "the view must be attached to a window");
        native
    }

    fn windows() -> Vec<Id> {
        // SAFETY: NSApp's window list contains live windows.
        unsafe {
            let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => Id);
            let windows = zpd_objc::msg_send!(app, zpd_objc::sel!("windows"), () => Id);
            let count = zpd_objc::msg_send!(windows, zpd_objc::sel!("count"), () => u64);
            (0..count)
                .map(|index| zpd_objc::msg_send!(windows, zpd_objc::sel!("objectAtIndex:"), ((index): u64) => Id))
                .collect()
        }
    }

    /// Finds a descendant by accessibility identifier.
    unsafe fn find(view: Id, identifier: &str) -> Id {
        // SAFETY: The caller passes a live NSView.
        unsafe {
            let current =
                zpd_objc::msg_send!(view, zpd_objc::sel!("accessibilityIdentifier"), () => Id);
            if !current.is_null() && rust_string(current) == identifier {
                return view;
            }
            let subviews = zpd_objc::msg_send!(view, zpd_objc::sel!("subviews"), () => Id);
            let count = zpd_objc::msg_send!(subviews, zpd_objc::sel!("count"), () => u64);
            for index in 0..count {
                let child = zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((index): u64) => Id);
                let found = Self::find(child, identifier);
                if !found.is_null() {
                    return found;
                }
            }
            zpd_objc::NIL
        }
    }
}

/// Reads an NSString as UTF-8.
/// Returns an empty `NSRange` at `location` in its C layout.
fn empty_range(location: u64) -> [u64; 2] {
    [location, 0]
}

/// Returns an autoreleased `NSString` for `value`.
///
/// # Safety
/// Must run on the main thread inside the test's autorelease scope.
unsafe fn ns_string(value: &str) -> Id {
    let bytes = std::ffi::CString::new(value).unwrap();
    // SAFETY: Guaranteed by the caller; the C string outlives the call.
    unsafe {
        zpd_objc::msg_send!(zpd_objc::class!("NSString"), zpd_objc::sel!("stringWithUTF8String:"), ((bytes.as_ptr()): *const std::ffi::c_char) => Id)
    }
}

unsafe fn rust_string(value: Id) -> String {
    // SAFETY: The caller passes a live NSString.
    unsafe {
        let bytes =
            zpd_objc::msg_send!(value, zpd_objc::sel!("UTF8String"), () => *const std::ffi::c_char);
        std::ffi::CStr::from_ptr(bytes)
            .to_string_lossy()
            .into_owned()
    }
}

fn main() {
    let application = Application::new().unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let _registration = application
        .on(move |event| observed.borrow_mut().push(event.kind))
        .unwrap();
    let window = application.create_window().unwrap();
    window
        .set_bounds(Rect::new(100.0, 100.0, 480.0, 360.0))
        .unwrap();
    window.show().unwrap();
    let content = window.content_view().unwrap();
    // Resize events depend on window chrome changes and are not part of these contracts.
    let take = || {
        let mut taken = std::mem::take(&mut *events.borrow_mut());
        taken.retain(|event| *event != WindowEventKind::DidResize);
        taken
    };
    take();

    // Verifies a switch reports its new state after a user toggle.
    let switch = Switch::new(&application).unwrap();
    content.add_subview(&switch).unwrap();
    // SAFETY: performClick: toggles a live NSSwitch and sends its action.
    unsafe {
        zpd_objc::msg_send!(Checks::native(&switch), zpd_objc::sel!("performClick:"), ((zpd_objc::NIL): Id) => ())
    };
    assert_eq!(take(), vec![WindowEventKind::StateChanged { on: true }]);
    assert!(switch.is_on().unwrap());

    // Verifies checkbox buttons report both activation and their state.
    let checkbox = Button::with_title(&application, "Check").unwrap();
    checkbox.set_button_type(ButtonType::Checkbox).unwrap();
    content.add_subview(&checkbox).unwrap();
    // SAFETY: performClick: toggles a live checkbox and sends its action.
    unsafe {
        zpd_objc::msg_send!(Checks::native(&checkbox), zpd_objc::sel!("performClick:"), ((zpd_objc::NIL): Id) => ())
    };
    assert_eq!(
        take(),
        vec![
            WindowEventKind::ButtonClicked,
            WindowEventKind::StateChanged { on: true }
        ]
    );

    // Verifies a title color colors the whole title, survives a new title, and can be removed.
    let colored = Button::with_title(&application, "Problem").unwrap();
    content.add_subview(&colored).unwrap();
    let red = Color::new(1.0, 0.0, 0.0, 1.0);
    colored.set_title_color(Some(red)).unwrap();
    assert_eq!(Checks::title_color(Checks::native(&colored)), Some(red));
    colored.set_title("Problem 2").unwrap();
    assert_eq!(Checks::title_color(Checks::native(&colored)), Some(red));
    // Without a color, AppKit draws the title in its own label color.
    colored.set_title_color(None).unwrap();
    assert_ne!(Checks::title_color(Checks::native(&colored)), Some(red));

    // Verifies sliders clamp to their range and report the value they send.
    let slider = Slider::new(&application).unwrap();
    slider.set_range(0.0, 10.0).unwrap();
    slider.set_value(4.0).unwrap();
    content.add_subview(&slider).unwrap();
    Checks::send_action(Checks::native(&slider));
    assert_eq!(take(), vec![WindowEventKind::ValueChanged { value: 4.0 }]);
    slider.set_value(20.0).unwrap();
    assert_eq!(slider.value().unwrap(), 10.0);

    // Verifies pop-up buttons keep duplicate titles and report the chosen index.
    let popup = PopUpButton::new(&application).unwrap();
    popup
        .set_items(&["Same".into(), "Same".into(), "Other".into()])
        .unwrap();
    popup.select(Some(2)).unwrap();
    content.add_subview(&popup).unwrap();
    let popup_native = Checks::native(&popup);
    // SAFETY: The live pop-up button reports its item count.
    let item_count =
        unsafe { zpd_objc::msg_send!(popup_native, zpd_objc::sel!("numberOfItems"), () => i64) };
    assert_eq!(item_count, 3);
    Checks::send_action(popup_native);
    assert_eq!(take(), vec![WindowEventKind::SelectionChanged { index: 2 }]);

    // Verifies segmented controls select one labeled segment.
    let segmented = SegmentedControl::new(&application).unwrap();
    segmented
        .set_segments(&["Day".into(), "Week".into(), "Month".into()])
        .unwrap();
    segmented.select(Some(1)).unwrap();
    content.add_subview(&segmented).unwrap();
    Checks::send_action(Checks::native(&segmented));
    assert_eq!(take(), vec![WindowEventKind::SelectionChanged { index: 1 }]);
    assert_eq!(segmented.selected().unwrap(), Some(1));

    // Verifies a field accessory is a button inside the field that reports clicks and narrows the text area.
    let text_width = |field: Id| {
        let bounds = NsRect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 22.0,
        };
        // SAFETY: The field is a live NSTextField whose cell computes its text area.
        unsafe {
            let cell = zpd_objc::msg_send!(field, zpd_objc::sel!("cell"), () => Id);
            zpd_objc::msg_send!(cell, zpd_objc::sel!("drawingRectForBounds:"), ((bounds): NsRect) => NsRect).width
        }
    };
    let accessory = FieldAccessory {
        system_image: "plus.forwardslash.minus".into(),
        label: "Calculator".into(),
        identifier: Some("amount-calculator".into()),
    };
    let field = TextField::with_string(&application, "").unwrap();
    field.as_view().set_frame(Rect::new(0.0, 0.0, 200.0, 22.0)).unwrap();
    content.add_subview(&field).unwrap();
    let field_native = Checks::native(&field);
    let plain_width = text_width(field_native);
    field.set_accessory(Some(&accessory)).unwrap();
    assert!(text_width(field_native) <= plain_width - 20.0);
    // SAFETY: The accessory is the field's only subview while it is not being edited.
    let button = unsafe {
        let subviews = zpd_objc::msg_send!(field_native, zpd_objc::sel!("subviews"), () => Id);
        zpd_objc::msg_send!(subviews, zpd_objc::sel!("lastObject"), () => Id)
    };
    // SAFETY: The button is live.
    let (identifier, refuses_focus, frame) = unsafe {
        (
            rust_string(zpd_objc::msg_send!(button, zpd_objc::sel!("accessibilityIdentifier"), () => Id)),
            zpd_objc::msg_send!(button, zpd_objc::sel!("refusesFirstResponder"), () => bool),
            zpd_objc::msg_send!(button, zpd_objc::sel!("frame"), () => NsRect),
        )
    };
    assert_eq!(identifier, "amount-calculator");
    assert!(refuses_focus);
    assert!(frame.x + frame.width <= 200.0 && frame.x >= 170.0);
    Checks::send_action(button);
    assert_eq!(take(), vec![WindowEventKind::AccessoryClicked]);
    // The field editor used while typing stops before the button too.
    field.focus().unwrap();
    // SAFETY: The focused field has a live field editor.
    let editor = unsafe {
        let editor = zpd_objc::msg_send!(field_native, zpd_objc::sel!("currentEditor"), () => Id);
        assert!(!editor.is_null());
        zpd_objc::msg_send!(editor, zpd_objc::sel!("frame"), () => NsRect)
    };
    assert!(editor.x + editor.width <= frame.x, "{editor:?} {frame:?}");
    // SAFETY: The field is in a live window; ending editing keeps later checks independent.
    unsafe {
        let window = zpd_objc::msg_send!(field_native, zpd_objc::sel!("window"), () => Id);
        zpd_objc::msg_send!(window, zpd_objc::sel!("makeFirstResponder:"), ((zpd_objc::NIL): Id) => bool);
    }
    take();
    // Removing the accessory hides the button and restores the text area.
    field.set_accessory(None).unwrap();
    assert_eq!(text_width(field_native), plain_width);
    // SAFETY: The button is live.
    assert!(unsafe { zpd_objc::msg_send!(button, zpd_objc::sel!("isHidden"), () => bool) });

    // Verifies Return and Escape confirm or cancel an input method's unconfirmed text instead of
    // clicking the default or cancel button, and click them once no text is unconfirmed.
    let memo = TextField::with_string(&application, "").unwrap();
    memo.as_view().set_frame(Rect::new(0.0, 60.0, 200.0, 22.0)).unwrap();
    content.add_subview(&memo).unwrap();
    let save = Button::with_title(&application, "Save").unwrap();
    save.set_key_equivalent("\r").unwrap();
    content.add_subview(&save).unwrap();
    let cancel = Button::with_title(&application, "Cancel").unwrap();
    cancel.set_key_equivalent("\u{1b}").unwrap();
    content.add_subview(&cancel).unwrap();
    let memo_native = Checks::native(&memo);
    memo.focus().unwrap();
    take();
    // SAFETY: The focused field has a live field editor in a live window; the events and
    // strings are autoreleased.
    unsafe {
        let window = zpd_objc::msg_send!(memo_native, zpd_objc::sel!("window"), () => Id);
        let editor = zpd_objc::msg_send!(memo_native, zpd_objc::sel!("currentEditor"), () => Id);
        assert!(!editor.is_null());
        let key = |characters: &str, code: u16| {
            let characters = ns_string(characters);
            let number = zpd_objc::msg_send!(window, zpd_objc::sel!("windowNumber"), () => isize);
            let origin = [0.0_f64, 0.0];
            zpd_objc::msg_send!(zpd_objc::class!("NSEvent"), zpd_objc::sel!("keyEventWithType:location:modifierFlags:timestamp:windowNumber:context:characters:charactersIgnoringModifiers:isARepeat:keyCode:"), ((10_u64): u64, (origin): [f64; 2], (0_u64): u64, (0.0): f64, (number): isize, (zpd_objc::NIL): Id, (characters): Id, (characters): Id, (false): bool, (code): u16) => Id)
        };
        let press = |event: Id| zpd_objc::msg_send!(window, zpd_objc::sel!("performKeyEquivalent:"), ((event): Id) => bool);
        let (enter, escape) = (key("\r", 36), key("\u{1b}", 53));
        let marked = ns_string("かいもの");
        let not_found = [i64::MAX as u64, 0];
        zpd_objc::msg_send!(editor, zpd_objc::sel!("setMarkedText:selectedRange:replacementRange:"), ((marked): Id, (empty_range(4)): [u64; 2], (not_found): [u64; 2]) => ());
        assert!(zpd_objc::msg_send!(editor, zpd_objc::sel!("hasMarkedText"), () => bool));
        assert!(!press(enter));
        assert!(!press(escape));
        assert!(take().is_empty());
        // Confirming the text leaves it in the field, and the next Return clicks the button.
        zpd_objc::msg_send!(editor, zpd_objc::sel!("unmarkText"), () => ());
        let text = zpd_objc::msg_send!(editor, zpd_objc::sel!("string"), () => Id);
        assert_eq!(rust_string(text), "かいもの");
        assert_eq!(take(), vec![WindowEventKind::TextChanged { value: "かいもの".into() }]);
        assert!(press(enter));
        assert_eq!(take(), vec![WindowEventKind::ButtonClicked]);
        assert!(press(escape));
        assert_eq!(take(), vec![WindowEventKind::ButtonClicked]);
        zpd_objc::msg_send!(window, zpd_objc::sel!("makeFirstResponder:"), ((zpd_objc::NIL): Id) => bool);
    }
    save.set_key_equivalent("").unwrap();
    cancel.set_key_equivalent("").unwrap();
    take();

    // Verifies stepper changes update the formatted field and report the value.
    let number = NumberField::new(&application).unwrap();
    number
        .set_format(NumberFormat {
            minimum: 0.0,
            maximum: 10.0,
            step: 2.0,
            fraction_digits: 0,
        })
        .unwrap();
    number.set_value(4.0).unwrap();
    content.add_subview(&number).unwrap();
    let number_native = Checks::native(&number);
    // SAFETY: The NumberField container owns the field and stepper subviews.
    let (field, stepper) = unsafe {
        let subviews = zpd_objc::msg_send!(number_native, zpd_objc::sel!("subviews"), () => Id);
        (
            zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => Id),
            zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((1): u64) => Id),
        )
    };
    // SAFETY: The stepper is live; the action mirrors a click on its up arrow.
    unsafe { zpd_objc::msg_send!(stepper, zpd_objc::sel!("setDoubleValue:"), ((6.0): f64) => ()) };
    Checks::send_action(stepper);
    assert_eq!(take(), vec![WindowEventKind::ValueChanged { value: 6.0 }]);
    // SAFETY: The field is a live NSTextField with a number formatter.
    let shown =
        unsafe { rust_string(zpd_objc::msg_send!(field, zpd_objc::sel!("stringValue"), () => Id)) };
    assert_eq!(shown, "6");
    // Verifies a number field's accessory sits inside its text field, before the stepper, and reports clicks from the field.
    let plain_width = text_width(field);
    number.set_accessory(Some(&accessory)).unwrap();
    assert!(text_width(field) <= plain_width - 20.0);
    // SAFETY: The accessory is the text field's only subview while it is not being edited.
    let button = unsafe {
        let subviews = zpd_objc::msg_send!(field, zpd_objc::sel!("subviews"), () => Id);
        zpd_objc::msg_send!(subviews, zpd_objc::sel!("lastObject"), () => Id)
    };
    Checks::send_action(button);
    assert_eq!(take(), vec![WindowEventKind::AccessoryClicked]);

    // Verifies a read-only number field hides its stepper, gives its text field the whole width,
    // keeps the text selectable, and stays a text field to accessibility.
    // SAFETY: The cell and views are live; the role is an autoreleased NSString.
    let role = |field: Id| unsafe {
        let cell = zpd_objc::msg_send!(field, zpd_objc::sel!("cell"), () => Id);
        rust_string(zpd_objc::msg_send!(cell, zpd_objc::sel!("accessibilityRole"), () => Id))
    };
    // SAFETY: The view is live.
    let width = |view: Id| unsafe { zpd_objc::msg_send!(view, zpd_objc::sel!("frame"), () => NsRect).width };
    let container_width = width(number_native);
    let editable_width = width(field);
    number.set_editable(false).unwrap();
    // SAFETY: The field and stepper are live.
    let (editable, selectable, stepper_hidden) = unsafe {
        (
            zpd_objc::msg_send!(field, zpd_objc::sel!("isEditable"), () => bool),
            zpd_objc::msg_send!(field, zpd_objc::sel!("isSelectable"), () => bool),
            zpd_objc::msg_send!(stepper, zpd_objc::sel!("isHidden"), () => bool),
        )
    };
    assert!(!editable && selectable && stepper_hidden);
    assert_eq!(width(field), container_width);
    assert_eq!(role(field), "AXTextField");
    number.set_editable(true).unwrap();
    // SAFETY: The stepper is live.
    assert!(!unsafe { zpd_objc::msg_send!(stepper, zpd_objc::sel!("isHidden"), () => bool) });
    assert_eq!(width(field), editable_width);

    // Verifies a read-only text field with a border stays a text field to accessibility, while a
    // label is static text, even after switching between them.
    let read_only = TextField::with_string(&application, "Expense").unwrap();
    content.add_subview(&read_only).unwrap();
    let read_only_native = Checks::native(&read_only);
    read_only.set_editable(false).unwrap();
    assert_eq!(role(read_only_native), "AXTextField");
    read_only.set_selectable(false).unwrap();
    read_only.set_bordered(false).unwrap();
    assert_eq!(role(read_only_native), "AXStaticText");
    read_only.set_editable(true).unwrap();
    read_only.set_bordered(true).unwrap();
    assert_eq!(role(read_only_native), "AXTextField");
    // A selectable label is still static text.
    let label = TextField::label_with_string(&application, "1,234").unwrap();
    content.add_subview(&label).unwrap();
    label.set_selectable(true).unwrap();
    assert_eq!(role(Checks::native(&label)), "AXStaticText");

    // Verifies the general pasteboard round-trips plain text; the previous text is put back.
    let previous = Pasteboard::string();
    Pasteboard::set_string("zintl pasteboard check");
    assert_eq!(Pasteboard::string().as_deref(), Some("zintl pasteboard check"));
    if let Some(previous) = previous {
        Pasteboard::set_string(&previous);
    }

    // Verifies dates round-trip through a date picker around month ends, leap days, and year boundaries.
    let picker = DatePicker::new(&application).unwrap();
    content.add_subview(&picker).unwrap();
    for date in [(2024, 2, 29), (2026, 1, 31), (2026, 12, 31), (2027, 1, 1), (1999, 3, 1)] {
        picker.set_date(date.0, date.1, date.2).unwrap();
        assert_eq!(picker.date().unwrap(), date);
    }
    // Programmatic dates are not reported as user changes.
    assert!(take().is_empty());
    picker.set_style(DatePickerStyle::Graphical).unwrap();
    assert_eq!(picker.date().unwrap(), (1999, 3, 1));

    // Verifies colors round-trip through a color well in sRGB.
    let well = ColorWell::new(&application).unwrap();
    content.add_subview(&well).unwrap();
    well.set_color(Color::new(1.0, 0.5, 0.0, 1.0)).unwrap();
    let color = well.color().unwrap();
    assert!((color.red - 1.0).abs() < 0.01 && (color.green - 0.5).abs() < 0.01);

    // Verifies a single-line field reports a line's height for layout, but no fixed width.
    let field = TextField::with_string(&application, "").unwrap();
    let (width, height) = field.as_view().intrinsic_content_size().unwrap();
    assert!(height.is_some_and(|height| height >= 20.0), "{height:?}");
    assert_eq!(width, None);

    // Verifies a number field reports its text field's height, though its container is a plain view.
    let number = NumberField::new(&application).unwrap();
    let (width, number_height) = number.intrinsic_content_size().unwrap();
    assert!(number_height >= height, "{number_height:?} < {height:?}");
    assert_eq!(width, None);

    // Verifies a wrapping label needs more lines at a narrower width.
    let long = "A long sentence that cannot fit on one line of a narrow pane and has to wrap.";
    let wrapping = TextField::label_with_string(&application, long).unwrap();
    wrapping.set_multiline(true).unwrap();
    let (natural_width, line) = wrapping.fitting_size(None).unwrap();
    let (_, narrow) = wrapping.fitting_size(Some(120.0)).unwrap();
    let (_, wider) = wrapping.fitting_size(Some(240.0)).unwrap();
    assert!(natural_width > 240.0, "{natural_width}");
    assert!(narrow > wider && wider > line, "{line} {wider} {narrow}");

    // Verifies label styling sets the font, semantic color, alignment, and tail truncation.
    let label = TextField::label_with_string(&application, "A long title").unwrap();
    content.add_subview(&label).unwrap();
    label
        .set_font(Some(&Font::new(None, 20.0, 700, false)))
        .unwrap();
    label.set_text_color(TextColor::SecondaryLabel).unwrap();
    label.set_alignment(TextAlignment::Center).unwrap();
    label.set_multiline(false).unwrap();
    label.set_line_limit(2).unwrap();
    let label_native = Checks::native(&label);
    // SAFETY: The label is a live NSTextField; the getters return live objects or scalars.
    unsafe {
        let font = zpd_objc::msg_send!(label_native, zpd_objc::sel!("font"), () => Id);
        assert_eq!(
            zpd_objc::msg_send!(font, zpd_objc::sel!("pointSize"), () => f64),
            20.0
        );
        let color = zpd_objc::msg_send!(label_native, zpd_objc::sel!("textColor"), () => Id);
        let secondary = zpd_objc::msg_send!(zpd_objc::class!("NSColor"), zpd_objc::sel!("secondaryLabelColor"), () => Id);
        assert!(zpd_objc::msg_send!(color, zpd_objc::sel!("isEqual:"), ((secondary): Id) => bool));
        let center = zpd_objc::msg_send!(label_native, zpd_objc::sel!("alignment"), () => isize);
        assert!(center == 1 || center == 2);
        assert_eq!(
            zpd_objc::msg_send!(label_native, zpd_objc::sel!("maximumNumberOfLines"), () => isize),
            2
        );
    }
    // Verifies system colors stay dynamic NSColors instead of fixed RGB values.
    label
        .set_text_color(TextColor::System(SystemColor::Red))
        .unwrap();
    // SAFETY: The label is a live NSTextField; the color getters return live objects.
    unsafe {
        let color = zpd_objc::msg_send!(label_native, zpd_objc::sel!("textColor"), () => Id);
        let red = zpd_objc::msg_send!(zpd_objc::class!("NSColor"), zpd_objc::sel!("systemRedColor"), () => Id);
        assert!(zpd_objc::msg_send!(color, zpd_objc::sel!("isEqual:"), ((red): Id) => bool));
    }
    label.set_font(None).unwrap();
    // SAFETY: The label is a live NSTextField with a font.
    let size = unsafe {
        let font = zpd_objc::msg_send!(label_native, zpd_objc::sel!("font"), () => Id);
        zpd_objc::msg_send!(font, zpd_objc::sel!("pointSize"), () => f64)
    };
    assert_eq!(size, 13.0);

    // Verifies lists host row views under group headers, select silently, and report user selection.
    let list = ListView::new(&application).unwrap();
    content.add_subview(&list).unwrap();
    list.as_view()
        .set_frame(Rect::new(0.0, 0.0, 240.0, 200.0))
        .unwrap();
    let first = View::new(&application, Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
    let second = View::new(&application, Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
    let entries = [
        ListEntry::Header("Pinned".into()),
        ListEntry::Row("a".into()),
        ListEntry::Header("Notes".into()),
        ListEntry::Row("b".into()),
    ];
    list.set_entries(&entries, &[first.as_view(), second.as_view()])
        .unwrap();
    // Replacing entries with headers again must leave the old cells intact.
    list.set_entries(&entries, &[first.as_view(), second.as_view()])
        .unwrap();
    list.set_row_heights(&[50.0, 30.0]).unwrap();
    list.select(Some("b")).unwrap();
    assert_eq!(list.selected_id().unwrap().as_deref(), Some("b"));
    assert!(take().is_empty());
    // SAFETY: The list's document view is its live NSTableView; table row 1 is row "a".
    unsafe {
        let scroll = Checks::native(&list);
        let table = zpd_objc::msg_send!(scroll, zpd_objc::sel!("documentView"), () => Id);
        assert_eq!(
            zpd_objc::msg_send!(table, zpd_objc::sel!("numberOfRows"), () => i64),
            4
        );
        let rect = zpd_objc::msg_send!(table, zpd_objc::sel!("rectOfRow:"), ((1): i64) => zpd_appkit::geometry::Rect);
        assert_eq!(rect.height, 50.0);
        let indexes = zpd_objc::msg_send!(zpd_objc::class!("NSIndexSet"), zpd_objc::sel!("indexSetWithIndex:"), ((1): u64) => Id);
        zpd_objc::msg_send!(table, zpd_objc::sel!("selectRowIndexes:byExtendingSelection:"), ((indexes): Id, (false): bool) => ());
    }
    assert_eq!(
        take(),
        vec![WindowEventKind::ListSelectionChanged {
            id: Some("a".into())
        }]
    );

    // Verifies declared column widths are the rendered widths, so they add up to the table width.
    let table = ListView::new(&application).unwrap();
    content.add_subview(&table).unwrap();
    table
        .as_view()
        .set_frame(Rect::new(0.0, 0.0, 320.0, 200.0))
        .unwrap();
    // Tables use the plain style, which has no insets beside the columns.
    table.set_style(ListStyle::Plain).unwrap();
    table
        .set_columns(&[
            TableColumn {
                title: "Name".into(),
                width: Some(200.0),
                alignment: TextAlignment::Leading,
                sort_key: Some("name".into()),
            },
            TableColumn {
                title: "Length".into(),
                width: Some(120.0),
                alignment: TextAlignment::Trailing,
                sort_key: None,
            },
        ])
        .unwrap();
    table.as_view().layout_subtree_if_needed().unwrap();
    // SAFETY: The list's document view is its live NSTableView with two columns.
    unsafe {
        let scroll = Checks::native(&table);
        let native = zpd_objc::msg_send!(scroll, zpd_objc::sel!("documentView"), () => Id);
        let widths = [0_i64, 1].map(|column| {
            zpd_objc::msg_send!(native, zpd_objc::sel!("rectOfColumn:"), ((column): i64) => zpd_appkit::geometry::Rect).width
        });
        assert_eq!(widths, [200.0, 120.0]);
        // Verifies a column's alignment reaches its header title.
        let columns = zpd_objc::msg_send!(native, zpd_objc::sel!("tableColumns"), () => Id);
        let length = zpd_objc::msg_send!(columns, zpd_objc::sel!("objectAtIndex:"), ((1): usize) => Id);
        let header = zpd_objc::msg_send!(length, zpd_objc::sel!("headerCell"), () => Id);
        let alignment = zpd_objc::msg_send!(header, zpd_objc::sel!("alignment"), () => isize);
        // NSTextAlignmentRight is 2 on Apple silicon and 1 on Intel Macs.
        assert!(alignment == 1 || alignment == 2, "{alignment}");
    }
    // Verifies a programmatic order shows the indicator on its column and reports nothing.
    table.set_sort(Some(("name", false))).unwrap();
    assert!(take().is_empty());
    // SAFETY: The list's document view is its live NSTableView with two columns.
    unsafe {
        let native = zpd_objc::msg_send!(Checks::native(&table), zpd_objc::sel!("documentView"), () => Id);
        let columns = zpd_objc::msg_send!(native, zpd_objc::sel!("tableColumns"), () => Id);
        let indicator = |index: usize| {
            let column = zpd_objc::msg_send!(columns, zpd_objc::sel!("objectAtIndex:"), ((index): usize) => Id);
            zpd_objc::msg_send!(native, zpd_objc::sel!("indicatorImageInTableColumn:"), ((column): Id) => Id)
        };
        assert!(!indicator(0).is_null());
        assert!(indicator(1).is_null());
    }

    // Verifies a summary row sits in a bar at the bottom, keeps rows above it, and lines up with the columns.
    let cells = [0, 1, 2, 3].map(|_| View::new(&application, Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap());
    table
        .set_entries(&[ListEntry::Row("a".into())], &[cells[0].as_view(), cells[1].as_view()])
        .unwrap();
    table.set_summary(&[cells[2].as_view(), cells[3].as_view()]).unwrap();
    assert!(table.has_summary());
    table.set_summary_height(24.0);
    table.as_view().layout_subtree_if_needed().unwrap();
    // SAFETY: The table's scroll view, its bar, and the summary cell wrappers are live.
    unsafe {
        let scroll = Checks::native(&table);
        let insets = zpd_objc::msg_send!(scroll, zpd_objc::sel!("contentInsets"), () => [f64; 4]);
        assert_eq!(insets[2], 25.0, "{insets:?}");
        let window_frame = |view: Id| {
            let bounds = zpd_objc::msg_send!(view, zpd_objc::sel!("bounds"), () => NsRect);
            zpd_objc::msg_send!(view, zpd_objc::sel!("convertRect:toView:"), ((bounds): NsRect, (zpd_objc::NIL): Id) => NsRect)
        };
        let wrapper = |view: &View| zpd_objc::msg_send!(Checks::native(view), zpd_objc::sel!("superview"), () => Id);
        let bar = zpd_objc::msg_send!(zpd_objc::msg_send!(wrapper(&cells[2]), zpd_objc::sel!("superview"), () => Id), zpd_objc::sel!("superview"), () => Id);
        let (scroll_frame, bar_frame) = (window_frame(scroll), window_frame(bar));
        // Window coordinates grow upward, so the bar's bottom is the scroll view's bottom.
        assert_eq!(bar_frame.y, scroll_frame.y, "{bar_frame:?} {scroll_frame:?}");
        assert_eq!(bar_frame.height, 25.0);
        let (name, length) = (window_frame(wrapper(&cells[2])), window_frame(wrapper(&cells[3])));
        let (row_name, row_length) = (window_frame(wrapper(&cells[0])), window_frame(wrapper(&cells[1])));
        assert_eq!((name.x, name.width), (row_name.x, row_name.width), "{name:?} {row_name:?}");
        assert_eq!((length.x, length.width), (row_length.x, row_length.width));
        assert_eq!(length.height, 24.0);
    }
    // Verifies re-rendering with the same cells keeps their wrappers and leaves the cells usable.
    // SAFETY: The summary cell is a live view inside its wrapper.
    let wrapper = |view: &View| unsafe { zpd_objc::msg_send!(Checks::native(view), zpd_objc::sel!("superview"), () => Id) };
    let before = [wrapper(&cells[2]), wrapper(&cells[3])];
    table
        .set_entries(&[ListEntry::Row("a".into())], &[cells[0].as_view(), cells[1].as_view()])
        .unwrap();
    table.set_summary(&[cells[2].as_view(), cells[3].as_view()]).unwrap();
    assert_eq!([wrapper(&cells[2]), wrapper(&cells[3])], before);
    // Verifies swapping in a new cell keeps the reused one alive in a new wrapper.
    let extra = View::new(&application, Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
    table.set_summary(&[cells[2].as_view(), extra.as_view()]).unwrap();
    assert_ne!(wrapper(&cells[2]), before[0]);
    // Removing it restores the insets.
    table.set_summary(&[]).unwrap();
    assert!(!table.has_summary());
    // Verifies a table without a summary row leaves the insets alone on later updates.
    table.set_summary(&[]).unwrap();
    assert!(!table.has_summary());

    // Verifies rich runs round-trip, and the paragraph action restyles undoably and validates its items.
    let rich = TextEditor::new(&application).unwrap();
    content.add_subview(&rich).unwrap();
    let runs = vec![
        RichRun {
            text: "Title\n".into(),
            style: RichStyle {
                paragraph: RichParagraph::Title,
                ..RichStyle::default()
            },
        },
        RichRun {
            text: "plain ".into(),
            style: RichStyle::default(),
        },
        RichRun {
            text: "bold".into(),
            style: RichStyle {
                bold: true,
                italic: true,
                underline: true,
                ..RichStyle::default()
            },
        },
    ];
    rich.set_rich_text(&runs).unwrap();
    assert_eq!(rich.rich_text().unwrap(), runs);
    assert!(take().is_empty());
    // SAFETY: The editor's document view is its live text view; the menu item is retained here.
    let (text, item) = unsafe {
        let text = zpd_objc::msg_send!(Checks::native(&rich), zpd_objc::sel!("documentView"), () => Id);
        zpd_objc::msg_send!(text, zpd_objc::sel!("setSelectedRange:"), ((empty_range(8)): [u64; 2]) => ());
        let item = zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSMenuItem"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithTitle:action:keyEquivalent:"), ((ns_string("Heading")): Id, (zpd_objc::sel!("zintlApplyParagraphStyle:")): zpd_objc::Sel, (ns_string("")): Id) => Id);
        zpd_objc::msg_send!(item, zpd_objc::sel!("setTag:"), ((RichParagraph::Heading.tag()): i64) => ());
        (text, item)
    };
    // SAFETY: The live text view validates and performs the action for the live item.
    unsafe {
        assert!(zpd_objc::msg_send!(text, zpd_objc::sel!("validateMenuItem:"), ((item): Id) => bool));
        assert_eq!(zpd_objc::msg_send!(item, zpd_objc::sel!("state"), () => isize), 0);
        zpd_objc::msg_send!(text, zpd_objc::sel!("zintlApplyParagraphStyle:"), ((item): Id) => ());
        assert!(zpd_objc::msg_send!(text, zpd_objc::sel!("validateMenuItem:"), ((item): Id) => bool));
        assert_eq!(zpd_objc::msg_send!(item, zpd_objc::sel!("state"), () => isize), 1);
    }
    let heading = |style: RichStyle| RichStyle {
        paragraph: RichParagraph::Heading,
        ..style
    };
    let mut expected = runs.clone();
    expected[1].style = heading(expected[1].style.clone());
    // A bold span adds nothing to a bold heading font, so it reads back without bold.
    expected[2].style = heading(RichStyle {
        bold: false,
        ..expected[2].style.clone()
    });
    assert_eq!(rich.rich_text().unwrap(), expected);
    assert_eq!(
        take(),
        vec![WindowEventKind::RichTextChanged { runs: expected }]
    );
    // SAFETY: The window's undo manager is live; the item is released after use.
    unsafe {
        let undo = zpd_objc::msg_send!(text, zpd_objc::sel!("undoManager"), () => Id);
        assert!(zpd_objc::msg_send!(undo, zpd_objc::sel!("canUndo"), () => bool));
        zpd_objc::msg_send!(undo, zpd_objc::sel!("undo"), () => ());
        zpd_objc::msg_send!(item, zpd_objc::sel!("release"), () => ());
    }
    assert_eq!(rich.rich_text().unwrap(), runs);
    take();

    // Verifies Return continues a checklist unchecked, then ends it on an empty item.
    let list = TextEditor::new(&application).unwrap();
    content.add_subview(&list).unwrap();
    list.set_rich_text(&[RichRun {
        text: "Milk".into(),
        style: RichStyle {
            paragraph: RichParagraph::Checklist { done: true },
            ..RichStyle::default()
        },
    }])
    .unwrap();
    take();
    // SAFETY: The editor's document view is its live text view; the caret is at the end.
    unsafe {
        let text = zpd_objc::msg_send!(Checks::native(&list), zpd_objc::sel!("documentView"), () => Id);
        zpd_objc::msg_send!(text, zpd_objc::sel!("insertNewline:"), ((zpd_objc::NIL): Id) => ());
        zpd_objc::msg_send!(text, zpd_objc::sel!("insertText:"), ((ns_string("Eggs")): Id) => ());
    }
    let paragraphs = |runs: Vec<RichRun>| {
        runs.into_iter()
            .map(|run| (run.text, run.style.paragraph))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        paragraphs(list.rich_text().unwrap()),
        [
            ("Milk\n".to_owned(), RichParagraph::Checklist { done: true }),
            ("Eggs".to_owned(), RichParagraph::Checklist { done: false }),
        ]
    );
    take();
    // SAFETY: The live text view handles Return and paragraph actions at the caret.
    unsafe {
        let text = zpd_objc::msg_send!(Checks::native(&list), zpd_objc::sel!("documentView"), () => Id);
        // Return on the empty item ends the list instead of adding a line.
        zpd_objc::msg_send!(text, zpd_objc::sel!("insertNewline:"), ((zpd_objc::NIL): Id) => ());
        zpd_objc::msg_send!(text, zpd_objc::sel!("insertNewline:"), ((zpd_objc::NIL): Id) => ());
        zpd_objc::msg_send!(text, zpd_objc::sel!("insertText:"), ((ns_string("Note")): Id) => ());
        // Return after a title continues in body text.
        let item = zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSMenuItem"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithTitle:action:keyEquivalent:"), ((ns_string("Title")): Id, (zpd_objc::sel!("zintlApplyParagraphStyle:")): zpd_objc::Sel, (ns_string("")): Id) => Id);
        zpd_objc::msg_send!(item, zpd_objc::sel!("setTag:"), ((RichParagraph::Title.tag()): i64) => ());
        zpd_objc::msg_send!(text, zpd_objc::sel!("zintlApplyParagraphStyle:"), ((item): Id) => ());
        zpd_objc::msg_send!(text, zpd_objc::sel!("insertNewline:"), ((zpd_objc::NIL): Id) => ());
        zpd_objc::msg_send!(text, zpd_objc::sel!("insertText:"), ((ns_string("Body")): Id) => ());
        zpd_objc::msg_send!(item, zpd_objc::sel!("release"), () => ());
    }
    assert_eq!(
        paragraphs(list.rich_text().unwrap()),
        [
            ("Milk\n".to_owned(), RichParagraph::Checklist { done: true }),
            ("Eggs\n".to_owned(), RichParagraph::Checklist { done: false }),
            ("Note\n".to_owned(), RichParagraph::Title),
            ("Body".to_owned(), RichParagraph::Body),
        ]
    );
    // A list action on a paragraph that already has it turns the list off.
    // SAFETY: The live text view performs the action on the caret paragraph.
    unsafe {
        let text = zpd_objc::msg_send!(Checks::native(&list), zpd_objc::sel!("documentView"), () => Id);
        let item = zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSMenuItem"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithTitle:action:keyEquivalent:"), ((ns_string("Checklist")): Id, (zpd_objc::sel!("zintlApplyParagraphStyle:")): zpd_objc::Sel, (ns_string("")): Id) => Id);
        zpd_objc::msg_send!(item, zpd_objc::sel!("setTag:"), ((RichParagraph::Checklist { done: false }.tag()): i64) => ());
        for _ in 0..2 {
            zpd_objc::msg_send!(text, zpd_objc::sel!("zintlApplyParagraphStyle:"), ((item): Id) => ());
        }
        zpd_objc::msg_send!(item, zpd_objc::sel!("release"), () => ());
    }
    assert_eq!(
        list.rich_text().unwrap().last().map(|run| run.style.paragraph),
        Some(RichParagraph::Body)
    );
    take();

    // Verifies controls inside list rows report events through the window.
    let rows = ListView::new(&application).unwrap();
    content.add_subview(&rows).unwrap();
    let row_button = Button::with_title(&application, "Row").unwrap();
    rows.as_view()
        .set_frame(Rect::new(0.0, 0.0, 200.0, 100.0))
        .unwrap();
    rows.set_entries(&[ListEntry::Row("row".into())], &[row_button.as_view()])
        .unwrap();
    rows.set_row_heights(&[30.0]).unwrap();
    // The table creates row views when it lays out.
    rows.as_view().layout_subtree_if_needed().unwrap();
    take();
    // SAFETY: performClick: sends the live button's action.
    unsafe {
        zpd_objc::msg_send!(Checks::native(&row_button), zpd_objc::sel!("performClick:"), ((zpd_objc::NIL): Id) => ())
    };
    assert_eq!(take(), vec![WindowEventKind::ButtonClicked]);
    // Verifies a double action without a clicked row, as below the last row, activates nothing.
    // SAFETY: The list's document view is its live table, which handles its double action.
    unsafe {
        let table = zpd_objc::msg_send!(Checks::native(&rows), zpd_objc::sel!("documentView"), () => Id);
        zpd_objc::msg_send!(table, zpd_objc::sel!("zpdActivateClickedRow:"), ((zpd_objc::NIL): Id) => ());
    }
    assert!(take().is_empty());

    // Verifies the editor reports typing as rich runs, replaces text silently, and drops stale undo history.
    let editor = TextEditor::new(&application).unwrap();
    content.add_subview(&editor).unwrap();
    assert!(editor.as_view().is_subview_of(content).unwrap());
    let body = |text: &str| RichRun {
        text: text.into(),
        style: RichStyle::default(),
    };
    editor.set_rich_text(&[body("Title\nBody")]).unwrap();
    assert_eq!(editor.rich_text().unwrap(), [body("Title\nBody")]);
    assert!(take().is_empty());
    // SAFETY: The editor's document view is its live NSTextView in a window with an undo manager.
    let undo = unsafe {
        let text = zpd_objc::msg_send!(Checks::native(&editor), zpd_objc::sel!("documentView"), () => Id);
        let typed = ns_string("!");
        zpd_objc::msg_send!(text, zpd_objc::sel!("insertText:"), ((typed): Id) => ());
        zpd_objc::msg_send!(text, zpd_objc::sel!("undoManager"), () => Id)
    };
    assert_eq!(
        take(),
        vec![WindowEventKind::RichTextChanged {
            runs: vec![body("Title\nBody!")]
        }]
    );
    // SAFETY: The window's undo manager is live.
    assert!(unsafe { zpd_objc::msg_send!(undo, zpd_objc::sel!("canUndo"), () => bool) });
    editor.set_rich_text(&[body("Other")]).unwrap();
    // SAFETY: The window's undo manager is live.
    assert!(!unsafe { zpd_objc::msg_send!(undo, zpd_objc::sel!("canUndo"), () => bool) });

    // Verifies scroll views host a flipped document that can grow past the viewport.
    let scroll = ScrollView::new(&application).unwrap();
    content.add_subview(&scroll).unwrap();
    scroll
        .as_view()
        .set_frame(Rect::new(0.0, 0.0, 200.0, 100.0))
        .unwrap();
    scroll.set_document_size(200.0, 400.0).unwrap();
    let (_, visible_height) = scroll.content_size().unwrap();
    assert!(visible_height <= 100.0);
    assert_eq!(scroll.document_view().bounds().unwrap().height, 400.0);

    // Verifies tab pages are installed in order and programmatic selection is silent.
    let tabs = TabView::new(&application).unwrap();
    content.add_subview(&tabs).unwrap();
    let first = View::new(&application, Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
    let second = View::new(&application, Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
    tabs.set_tabs(&[("General", first.as_view()), ("Advanced", second.as_view())])
        .unwrap();
    tabs.select(1).unwrap();
    assert_eq!(tabs.selected().unwrap(), Some(1));
    assert!(take().is_empty());
    tabs.set_tabs(&[("Only", first.as_view())]).unwrap();
    assert_eq!(tabs.selected().unwrap(), Some(0));

    // Verifies context menus report custom items and detach when dropped.
    let anchor = Button::with_title(&application, "Menu").unwrap();
    content.add_subview(&anchor).unwrap();
    let mut rename = CommandItem::new("Rename");
    rename.id = Some("rename".into());
    let menu = ContextMenu::install(
        anchor.as_view(),
        &[
            rename,
            CommandItem::separator(),
            CommandItem::new("Disabled"),
        ],
        None,
    )
    .unwrap();
    let anchor_native = Checks::native(&anchor);
    // SAFETY: The anchor owns its contextual NSMenu while the ContextMenu lives.
    unsafe {
        let native_menu = zpd_objc::msg_send!(anchor_native, zpd_objc::sel!("menu"), () => Id);
        assert_eq!(
            zpd_objc::msg_send!(native_menu, zpd_objc::sel!("numberOfItems"), () => i64),
            3
        );
        zpd_objc::msg_send!(native_menu, zpd_objc::sel!("performActionForItemAtIndex:"), ((0): i64) => ());
    }
    assert_eq!(
        take(),
        vec![WindowEventKind::MenuItemSelected {
            id: "rename".into()
        }]
    );
    drop(menu);
    // SAFETY: The anchor is live after its menu was removed.
    let cleared =
        unsafe { zpd_objc::msg_send!(anchor_native, zpd_objc::sel!("menu"), () => Id).is_null() };
    assert!(cleared);

    // Verifies popovers own a content view below their anchor.
    let popover = Popover::new(anchor.as_view()).unwrap();
    assert!(popover.content_view().bounds().is_ok());
    assert!(!popover.is_shown());
    drop(popover);

    // Verifies a sidebar search field survives item updates and reports edits.
    let sidebar = |items: &[&str], value: &str| Sidebar {
        sections: vec![SidebarSection {
            title: None,
            items: items
                .iter()
                .map(|id| SidebarItem {
                    id: (*id).into(),
                    title: (*id).into(),
                    system_image: None,
                    badge: None,
                    menu: Vec::new(),
                })
                .collect(),
        }],
        selected_id: None,
        search: Some(SidebarSearch {
            value: value.into(),
            placeholder: Some("Search".into()),
        }),
    };
    window.update_sidebar(&sidebar(&["a", "b"], "")).unwrap();
    let search_field = || {
        // SAFETY: The window owns a split view whose sidebar controller hosts the search field.
        unsafe {
            let window =
                zpd_objc::msg_send!(Checks::native(&anchor), zpd_objc::sel!("window"), () => Id);
            let split =
                zpd_objc::msg_send!(window, zpd_objc::sel!("contentViewController"), () => Id);
            let items = zpd_objc::msg_send!(split, zpd_objc::sel!("splitViewItems"), () => Id);
            let item =
                zpd_objc::msg_send!(items, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => Id);
            let controller = zpd_objc::msg_send!(item, zpd_objc::sel!("viewController"), () => Id);
            let container = zpd_objc::msg_send!(controller, zpd_objc::sel!("view"), () => Id);
            let subviews = zpd_objc::msg_send!(container, zpd_objc::sel!("subviews"), () => Id);
            zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => Id)
        }
    };
    let field_before = search_field();
    window.update_sidebar(&sidebar(&["a"], "a")).unwrap();
    assert_eq!(search_field(), field_before);
    // SAFETY: The search field is live; its action reports the current text.
    let value = unsafe {
        rust_string(zpd_objc::msg_send!(field_before, zpd_objc::sel!("stringValue"), () => Id))
    };
    assert_eq!(value, "a");
    Checks::send_action(field_before);
    assert_eq!(
        take(),
        vec![WindowEventKind::SidebarSearchChanged { value: "a".into() }]
    );

    // Verifies alert sheets end with no button when dismissed programmatically.
    let alert = AlertSheet::present(
        &window,
        &AlertConfig {
            title: "Delete?".into(),
            buttons: vec![
                AlertButton {
                    title: "Delete".into(),
                    role: AlertButtonRole::Destructive,
                },
                AlertButton {
                    title: "Cancel".into(),
                    role: AlertButtonRole::Cancel,
                },
            ],
            ..AlertConfig::default()
        },
        None,
    )
    .unwrap();
    // Verifies alert buttons also leave Return and Escape to an input method.
    /// Counts the buttons under `view` that use the input-method-aware class.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    unsafe fn key_buttons(view: Id) -> usize {
        // SAFETY: Guaranteed by the caller; subviews of live views are live.
        unsafe {
            let class = zpd_objc::ffi::object_getClass(view);
            let name = std::ffi::CStr::from_ptr(zpd_objc::ffi::class_getName(class));
            let own = usize::from(name.to_bytes().starts_with(b"ZpdKey_"));
            let subviews = zpd_objc::msg_send!(view, zpd_objc::sel!("subviews"), () => Id);
            let count = zpd_objc::msg_send!(subviews, zpd_objc::sel!("count"), () => u64);
            own + (0..count)
                .map(|index| key_buttons(zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((index): u64) => Id)))
                .sum::<usize>()
        }
    }
    // SAFETY: The main window is live and its attached sheet is the alert's live window.
    unsafe {
        let main = zpd_objc::msg_send!(memo_native, zpd_objc::sel!("window"), () => Id);
        let sheet = zpd_objc::msg_send!(main, zpd_objc::sel!("attachedSheet"), () => Id);
        assert!(!sheet.is_null());
        let sheet_content = zpd_objc::msg_send!(sheet, zpd_objc::sel!("contentView"), () => Id);
        assert_eq!(key_buttons(sheet_content), 2);
    }
    alert.dismiss();
    drop(alert);
    assert_eq!(
        take(),
        vec![WindowEventKind::AlertResponse { button: None }]
    );

    // Verifies panels use NSPanel with floating utility behavior.
    let panel = application
        .create_styled_window(None, WindowStyle::Panel)
        .unwrap();
    assert!(!panel.is_closed());
    let floating = Checks::windows().into_iter().any(|window| {
        // SAFETY: Each window is live; only NSPanel instances receive isFloatingPanel.
        unsafe {
            zpd_objc::msg_send!(window, zpd_objc::sel!("isKindOfClass:"), ((zpd_objc::class!("NSPanel")): Id) => bool)
                && zpd_objc::msg_send!(window, zpd_objc::sel!("isFloatingPanel"), () => bool)
        }
    });
    assert!(floating);

    // Verifies toolbar buttons report clicks and state changes update items in place.
    let button = |enabled| ToolbarItem::Button {
        id: "new".into(),
        label: "New".into(),
        system_image: "plus".into(),
        enabled,
    };
    let declared = |enabled| Toolbar {
        items: vec![
            button(enabled),
            ToolbarItem::FlexibleSpace,
            ToolbarItem::Search {
                id: Some("toolbar-search".into()),
                value: String::new(),
                placeholder: Some("Search".into()),
            },
        ],
    };
    take();
    window.set_toolbar(Some(&declared(true))).unwrap();
    // SAFETY: The window owns its toolbar; items and the action target are live.
    let item = unsafe {
        let windows = Checks::windows();
        let toolbar = windows
            .iter()
            .map(|window| zpd_objc::msg_send!(*window, zpd_objc::sel!("toolbar"), () => Id))
            .find(|toolbar| !toolbar.is_null())
            .expect("the window shows a toolbar");
        // The window has a sidebar here, so its toggle and separator come first.
        let items = zpd_objc::msg_send!(toolbar, zpd_objc::sel!("items"), () => Id);
        assert_eq!(zpd_objc::msg_send!(items, zpd_objc::sel!("count"), () => u64), 5);
        let item = zpd_objc::msg_send!(items, zpd_objc::sel!("objectAtIndex:"), ((2): u64) => Id);
        let identifier = zpd_objc::msg_send!(item, zpd_objc::sel!("itemIdentifier"), () => Id);
        assert_eq!(rust_string(identifier), "zintl.toolbar.0");
        let action = zpd_objc::msg_send!(item, zpd_objc::sel!("action"), () => zpd_objc::Sel);
        let target = zpd_objc::msg_send!(item, zpd_objc::sel!("target"), () => Id);
        zpd_objc::msg_send!(target, action, ((item): Id) => ());
        // Verifies the search field carries its declared ID for UI tests.
        let search = zpd_objc::msg_send!(items, zpd_objc::sel!("objectAtIndex:"), ((4): u64) => Id);
        let field = zpd_objc::msg_send!(search, zpd_objc::sel!("searchField"), () => Id);
        let field_id = zpd_objc::msg_send!(field, zpd_objc::sel!("accessibilityIdentifier"), () => Id);
        assert_eq!(rust_string(field_id), "toolbar-search");
        item
    };
    assert_eq!(
        take(),
        vec![WindowEventKind::ToolbarItemClicked { id: "new".into() }]
    );
    window.set_toolbar(Some(&declared(false))).unwrap();
    // SAFETY: The same item stays installed when only its state changes.
    assert!(!unsafe { zpd_objc::msg_send!(item, zpd_objc::sel!("isEnabled"), () => bool) });
    window.set_toolbar(None).unwrap();

    // Verifies a button installed disabled stays disabled once its action is set.
    window.set_toolbar(Some(&declared(false))).unwrap();
    // SAFETY: The window owns its toolbar and the items it lists.
    let enabled = unsafe {
        let toolbar = Checks::windows()
            .iter()
            .map(|window| zpd_objc::msg_send!(*window, zpd_objc::sel!("toolbar"), () => Id))
            .find(|toolbar| !toolbar.is_null())
            .expect("the window shows a toolbar");
        let items = zpd_objc::msg_send!(toolbar, zpd_objc::sel!("items"), () => Id);
        let item = zpd_objc::msg_send!(items, zpd_objc::sel!("objectAtIndex:"), ((2): u64) => Id);
        zpd_objc::msg_send!(item, zpd_objc::sel!("isEnabled"), () => bool)
    };
    assert!(!enabled);
    window.set_toolbar(None).unwrap();

    // Verifies the will-terminate hook runs when AppKit announces that it is exiting.
    let exited = Rc::new(std::cell::Cell::new(0));
    let count = exited.clone();
    application
        .on_will_terminate(move || count.set(count.get() + 1))
        .unwrap();
    // SAFETY: Posting the notification on the main thread runs observers synchronously.
    unsafe {
        let center = zpd_objc::msg_send!(zpd_objc::class!("NSNotificationCenter"), zpd_objc::sel!("defaultCenter"), () => Id);
        zpd_objc::msg_send!(center, zpd_objc::sel!("postNotificationName:object:"), ((ns_string("NSApplicationWillTerminateNotification")): Id, (zpd_objc::NIL): Id) => ());
    }
    assert_eq!(exited.get(), 1);

    // Verifies standard roles use responder-chain actions and register NSApp menus.
    application
        .set_commands(
            &CommandSet {
                app_menu: None,
                menus: vec![CommandMenu {
                    title: "Services".into(),
                    items: Vec::new(),
                    role: Some(MenuRole::Services),
                }],
            },
            |_| {},
        )
        .unwrap();
    let mut copy = CommandItem::new("Copy");
    copy.role = Some(CommandRole::Copy);
    // Find items carry their text finder action as a tag; Bold targets the font manager.
    let roles = [
        (CommandRole::Find, zpd_objc::sel!("performTextFinderAction:"), 1),
        (CommandRole::FindNext, zpd_objc::sel!("performTextFinderAction:"), 2),
        (CommandRole::Bold, zpd_objc::sel!("addFontTrait:"), 2),
        (CommandRole::Underline, zpd_objc::sel!("underline:"), 0),
        (
            CommandRole::Checklist,
            zpd_objc::sel!("zintlApplyParagraphStyle:"),
            8,
        ),
    ];
    let mut items = vec![copy];
    items.extend(roles.iter().map(|(role, _, _)| {
        let mut item = CommandItem::new(format!("{role:?}"));
        item.role = Some(*role);
        item
    }));
    application
        .set_commands(
            &CommandSet {
                app_menu: None,
                menus: vec![CommandMenu {
                    title: "Edit".into(),
                    items,
                    role: None,
                }],
            },
            |_| {},
        )
        .unwrap();
    // SAFETY: NSApp's main menu and its items are live after installation.
    unsafe {
        let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => Id);
        let main = zpd_objc::msg_send!(app, zpd_objc::sel!("mainMenu"), () => Id);
        let count = zpd_objc::msg_send!(main, zpd_objc::sel!("numberOfItems"), () => i64);
        let edit = zpd_objc::msg_send!(zpd_objc::msg_send!(main, zpd_objc::sel!("itemAtIndex:"), ((count - 1): i64) => Id), zpd_objc::sel!("submenu"), () => Id);
        let font_manager = zpd_objc::msg_send!(zpd_objc::class!("NSFontManager"), zpd_objc::sel!("sharedFontManager"), () => Id);
        for (index, (role, action, tag)) in roles.iter().enumerate() {
            let item = zpd_objc::msg_send!(edit, zpd_objc::sel!("itemAtIndex:"), ((index as i64 + 1): i64) => Id);
            let selector = zpd_objc::msg_send!(item, zpd_objc::sel!("action"), () => zpd_objc::Sel);
            assert_eq!(selector, *action, "{role:?}");
            assert_eq!(zpd_objc::msg_send!(item, zpd_objc::sel!("tag"), () => i64), *tag, "{role:?}");
            let target = zpd_objc::msg_send!(item, zpd_objc::sel!("target"), () => Id);
            let expected = if *role == CommandRole::Bold { font_manager } else { zpd_objc::NIL };
            assert_eq!(target, expected, "{role:?}");
        }
    }
}
