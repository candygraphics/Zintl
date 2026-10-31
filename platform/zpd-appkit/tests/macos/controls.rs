// Runs native control, menu, and dialog contracts on AppKit's main thread.
use std::cell::RefCell;
use std::rc::Rc;

use zpd_appkit::actor::WindowEventKind;
use zpd_appkit::geometry::Rect;
use zpd_appkit::graphics::Font;
use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AlertButton, AlertButtonRole, AlertConfig, AlertSheet, AsView, Button, ButtonType, Color,
    ColorWell, ListEntry, ListView, CommandItem, CommandMenu, CommandRole, CommandSet, ContextMenu, MenuRole,
    NumberField, NumberFormat, PopUpButton, Popover, ScrollView, SegmentedControl, Sidebar,
    SidebarItem, SidebarSearch, SidebarSection, Slider, Switch, TabView, TextAlignment, TextColor,
    RichParagraph, RichRun, RichStyle, TextEditor, TextField, Toolbar, ToolbarItem, View, WindowStyle,
};
use zpd_objc::Id;

struct Checks;

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

    // Verifies colors round-trip through a color well in sRGB.
    let well = ColorWell::new(&application).unwrap();
    content.add_subview(&well).unwrap();
    well.set_color(Color::new(1.0, 0.5, 0.0, 1.0)).unwrap();
    let color = well.color().unwrap();
    assert!((color.red - 1.0).abs() < 0.01 && (color.green - 0.5).abs() < 0.01);

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

    // Verifies rich runs round-trip, and the paragraph action restyles undoably and validates its items.
    let rich = TextEditor::new(&application).unwrap();
    content.add_subview(&rich).unwrap();
    rich.set_rich(true).unwrap();
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
    // Plain editors disable paragraph styles.
    rich.set_rich(false).unwrap();
    // SAFETY: The live text view validates a live item.
    unsafe {
        let item = zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSMenuItem"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("initWithTitle:action:keyEquivalent:"), ((ns_string("Body")): Id, (zpd_objc::sel!("zintlApplyParagraphStyle:")): zpd_objc::Sel, (ns_string("")): Id) => Id);
        assert!(!zpd_objc::msg_send!(text, zpd_objc::sel!("validateMenuItem:"), ((item): Id) => bool));
        zpd_objc::msg_send!(item, zpd_objc::sel!("release"), () => ());
    }

    // Verifies Return continues a checklist unchecked, then ends it on an empty item.
    let list = TextEditor::new(&application).unwrap();
    content.add_subview(&list).unwrap();
    list.set_rich(true).unwrap();
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

    // Verifies the editor reports user edits, replaces text silently, and drops stale undo history.
    let editor = TextEditor::new(&application).unwrap();
    content.add_subview(&editor).unwrap();
    assert!(editor.as_view().is_subview_of(content).unwrap());
    editor.set_string_value("Title\nBody").unwrap();
    assert_eq!(editor.string_value().unwrap(), "Title\nBody");
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
        vec![WindowEventKind::TextChanged {
            value: "Title\nBody!".into()
        }]
    );
    // SAFETY: The window's undo manager is live.
    assert!(unsafe { zpd_objc::msg_send!(undo, zpd_objc::sel!("canUndo"), () => bool) });
    editor.set_string_value("Other").unwrap();
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
