// Runs NSAccessibility attribute contracts on AppKit's main thread.
#[path = "../../src/native.rs"]
mod native;

use zpd_appkit::geometry::Rect;
use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AccessibilityAnnouncement, AccessibilityAttributes, AccessibilityNotification,
    AccessibilityRole, AnnouncementPriority, AsView, Button, ListView, NumberField, Slider,
    TextEditor, TextField, View, ViewRef,
};
use zpd_objc::Id;

/// Reads accessibility attributes back from live views.
struct Checks {
    content: Id,
}

impl Checks {
    /// Returns the native view of `view` through its accessibility identifier.
    fn native(&self, view: ViewRef<'_>, identifier: &str) -> Id {
        view.set_identifier(Some(identifier)).unwrap();
        // SAFETY: The content view is live on the main thread.
        let native = unsafe { Self::find(self.content, identifier) };
        assert!(!native.is_null(), "{identifier} must be attached to the window");
        native
    }

    /// Finds a descendant by accessibility identifier.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    unsafe fn find(view: Id, identifier: &str) -> Id {
        // SAFETY: Guaranteed by the caller; subviews of live views are live.
        unsafe {
            let current =
                zpd_objc::msg_send!(view, zpd_objc::sel!("accessibilityIdentifier"), () => Id);
            if !current.is_null() && native::rust_string(current) == identifier {
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

    /// Returns a string attribute of `view` read through `getter`, or `None` when nil.
    fn string(view: Id, getter: zpd_objc::Sel) -> Option<String> {
        // SAFETY: The view is live and the getter returns an NSString or nil.
        unsafe {
            let value = zpd_objc::msg_send!(view, getter, () => Id);
            (!value.is_null()).then(|| native::rust_string(value))
        }
    }

    fn label(view: Id) -> Option<String> {
        Self::string(view, zpd_objc::sel!("accessibilityLabel"))
    }

    fn is_element(view: Id) -> bool {
        // SAFETY: The view is live and implements NSAccessibility.
        unsafe { zpd_objc::msg_send!(view, zpd_objc::sel!("isAccessibilityElement"), () => bool) }
    }

    /// Returns the cell VoiceOver visits for a cell-based control.
    fn cell(view: Id) -> Id {
        // SAFETY: The view is a live NSControl.
        unsafe { zpd_objc::msg_send!(view, zpd_objc::sel!("cell"), () => Id) }
    }

    /// Returns whether `element` is among the content view's accessibility children.
    fn is_child(&self, element: Id) -> bool {
        // SAFETY: The content view is live; its accessibility children form an NSArray.
        unsafe {
            let children =
                zpd_objc::msg_send!(self.content, zpd_objc::sel!("accessibilityChildren"), () => Id);
            zpd_objc::msg_send!(children, zpd_objc::sel!("containsObject:"), ((element): Id) => bool)
        }
    }

    fn is_kind(object: Id, class: &str) -> bool {
        let class = std::ffi::CString::new(class).unwrap();
        // SAFETY: The object is live; objc_getClass returns a registered class.
        unsafe {
            let class = zpd_objc::ffi::objc_getClass(class.as_ptr());
            zpd_objc::msg_send!(object, zpd_objc::sel!("isKindOfClass:"), ((class): zpd_objc::Class) => bool)
        }
    }
}

fn main() {
    let application = Application::new().unwrap();
    let window = application.create_window().unwrap();
    window.set_bounds(Rect::new(100.0, 100.0, 480.0, 360.0)).unwrap();
    window.show().unwrap();
    let content = window.content_view().unwrap();
    // SAFETY: This isolated main-thread test creates exactly one NSWindow.
    let checks = Checks {
        content: unsafe {
            let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => Id);
            let windows = zpd_objc::msg_send!(app, zpd_objc::sel!("windows"), () => Id);
            let window = zpd_objc::msg_send!(windows, zpd_objc::sel!("objectAtIndex:"), ((0): u64) => Id);
            zpd_objc::msg_send!(window, zpd_objc::sel!("contentView"), () => Id)
        },
    };

    let button = Button::with_title(&application, "Save").unwrap();
    content.add_subview(&button).unwrap();
    let button_native = checks.native(button.as_view(), "button");
    let plain_control = AccessibilityAttributes {
        element: true,
        ..AccessibilityAttributes::default()
    };
    let labeled = AccessibilityAttributes {
        label: Some("Store".into()),
        help: Some("Writes the file".into()),
        ..plain_control.clone()
    };
    button
        .as_view()
        .set_accessibility(&plain_control, &labeled)
        .unwrap();
    // Verifies a label and help replace what a button reports by itself.
    assert_eq!(Checks::label(button_native).as_deref(), Some("Store"));
    assert_eq!(
        Checks::string(button_native, zpd_objc::sel!("accessibilityHelp")).as_deref(),
        Some("Writes the file")
    );
    button
        .as_view()
        .set_accessibility(&labeled, &plain_control)
        .unwrap();
    // Verifies clearing the label leaves the title for VoiceOver to read.
    assert_eq!(Checks::label(button_native), None);
    assert_eq!(
        Checks::string(button_native, zpd_objc::sel!("accessibilityTitle")).as_deref(),
        Some("Save")
    );
    button
        .as_view()
        .set_accessibility(&plain_control, &AccessibilityAttributes::default())
        .unwrap();
    // Verifies a hidden control and the cell VoiceOver visits are no longer elements.
    assert!(!Checks::is_element(button_native));
    assert!(!Checks::is_element(Checks::cell(button_native)));
    assert!(!checks.is_child(Checks::cell(button_native)));

    let container = View::new(&application, Rect::new(0.0, 0.0, 100.0, 40.0)).unwrap();
    content.add_subview(&container).unwrap();
    let container_native = checks.native(container.as_view(), "container");
    // Verifies plain containers are skipped by VoiceOver.
    assert!(!Checks::is_element(container_native));
    container
        .as_view()
        .set_accessibility(&AccessibilityAttributes::default(), &AccessibilityAttributes {
            element: true,
            label: Some("Toolbox".into()),
            value: Some("3 tools".into()),
            replaces_value: true,
            role: Some(AccessibilityRole::Group),
            ..AccessibilityAttributes::default()
        })
        .unwrap();
    // Verifies a named container reads as a group with its own value.
    assert!(Checks::is_element(container_native));
    assert_eq!(
        Checks::string(container_native, zpd_objc::sel!("accessibilityRole")).as_deref(),
        Some("AXGroup")
    );
    assert_eq!(Checks::label(container_native).as_deref(), Some("Toolbox"));
    assert_eq!(
        Checks::string(container_native, zpd_objc::sel!("accessibilityValue")).as_deref(),
        Some("3 tools")
    );
    container
        .as_view()
        .post_accessibility_notification(AccessibilityNotification::LayoutChanged)
        .unwrap();

    let slider = Slider::new(&application).unwrap();
    slider.set_range(0.0, 1.0).unwrap();
    slider.set_value(0.5).unwrap();
    content.add_subview(&slider).unwrap();
    let slider_native = checks.native(slider.as_view(), "slider");
    slider
        .as_view()
        .set_accessibility(
            &plain_control,
            &AccessibilityAttributes {
                value: Some("Half".into()),
                ..plain_control.clone()
            },
        )
        .unwrap();
    // Verifies a control's value gets a spoken description instead of being replaced.
    assert_eq!(
        Checks::string(slider_native, zpd_objc::sel!("accessibilityValueDescription")).as_deref(),
        Some("Half")
    );
    assert_eq!(
        Checks::string(Checks::cell(slider_native), zpd_objc::sel!("accessibilityValueDescription"))
            .as_deref(),
        Some("Half")
    );
    slider
        .as_view()
        .post_accessibility_notification(AccessibilityNotification::ValueChanged)
        .unwrap();

    let label = TextField::label_with_string(&application, "Speed").unwrap();
    content.add_subview(&label).unwrap();
    let label_native = checks.native(label.as_view(), "label");
    slider
        .as_view()
        .set_accessibility_title_element(Some(label.as_view()))
        .unwrap();
    // SAFETY: The slider cell is live; the title element is an object or nil.
    let title = unsafe {
        zpd_objc::msg_send!(Checks::cell(slider_native), zpd_objc::sel!("accessibilityTitleUIElement"), () => Id)
    };
    // Verifies the slider cell VoiceOver visits is named by the label's cell.
    assert_eq!(title, Checks::cell(label_native));
    slider.as_view().set_accessibility_title_element(None).unwrap();
    // SAFETY: As above.
    let title = unsafe {
        zpd_objc::msg_send!(slider_native, zpd_objc::sel!("accessibilityTitleUIElement"), () => Id)
    };
    // Verifies the link can be removed.
    assert!(title.is_null());

    let editor = TextEditor::new(&application).unwrap();
    let list = ListView::new(&application).unwrap();
    let number = NumberField::new(&application).unwrap();
    content.add_subview(&editor).unwrap();
    content.add_subview(&list).unwrap();
    content.add_subview(&number).unwrap();
    // Verifies wrapped controls expose the inner view VoiceOver reads.
    assert!(Checks::is_kind(
        checks.native(editor.accessibility_view(), "editor"),
        "NSTextView"
    ));
    assert!(Checks::is_kind(
        checks.native(list.accessibility_view(), "list"),
        "NSTableView"
    ));
    assert!(Checks::is_kind(
        checks.native(number.accessibility_view(), "number"),
        "NSTextField"
    ));

    // Verifies posting an announcement does not need a focused element.
    AccessibilityAnnouncement::post("Saved", AnnouncementPriority::High);
}
