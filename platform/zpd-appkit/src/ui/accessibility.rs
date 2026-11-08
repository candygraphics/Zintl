//! `NSAccessibility` attributes and notifications that VoiceOver reads.

use zpd_objc::{Id, Strong};

use crate::native;

use super::UiError;
use super::view::ViewRef;

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    fn NSAccessibilityPostNotification(element: Id, notification: Id);
    fn NSAccessibilityPostNotificationWithUserInfo(element: Id, notification: Id, info: Id);
    static NSAccessibilityValueChangedNotification: Id;
    static NSAccessibilityLayoutChangedNotification: Id;
    static NSAccessibilityAnnouncementRequestedNotification: Id;
    static NSAccessibilityAnnouncementKey: Id;
    static NSAccessibilityPriorityKey: Id;
}

/// A role for views without a fitting native control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
    Group,
    Image,
    StaticText,
}

impl AccessibilityRole {
    /// Returns the `NSAccessibilityRole` string.
    fn name(self) -> &'static str {
        match self {
            Self::Group => "AXGroup",
            Self::Image => "AXImage",
            Self::StaticText => "AXStaticText",
        }
    }
}

/// The attributes [`ViewRef::set_accessibility`] writes.
///
/// `None` fields leave what the view reports by itself, such as a button's title.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessibilityAttributes {
    /// Whether VoiceOver visits the view as an element of its own.
    pub element: bool,
    pub label: Option<String>,
    pub value: Option<String>,
    /// Whether `value` replaces the view's value instead of describing it.
    ///
    /// Set it for views without a native value, such as containers; controls
    /// keep their value and speak `value` as its description.
    pub replaces_value: bool,
    pub help: Option<String>,
    pub role: Option<AccessibilityRole>,
}

/// A change that VoiceOver should notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityNotification {
    /// The view's value changed.
    ValueChanged,
    /// Elements inside the view appeared, disappeared, or moved.
    LayoutChanged,
}

/// How urgently an announcement interrupts speech.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnouncementPriority {
    Low,
    Medium,
    High,
}

/// Speech that VoiceOver says without moving focus.
pub struct AccessibilityAnnouncement;

impl AccessibilityAnnouncement {
    /// Asks VoiceOver to speak `text`.
    pub fn post(text: &str, priority: AnnouncementPriority) {
        // NSAccessibilityPriorityLevel values.
        let level: native::Integer = match priority {
            AnnouncementPriority::Low => 10,
            AnnouncementPriority::Medium => 50,
            AnnouncementPriority::High => 90,
        };
        let text = native::nsstring(text);
        // SAFETY: The constants are AppKit NSStrings; the dictionary copies its live
        // arguments, and NSApp is the shared application on the main thread.
        unsafe {
            let priority = zpd_objc::msg_send!(zpd_objc::class!("NSNumber"), zpd_objc::sel!("numberWithInteger:"), ((level): native::Integer) => Id);
            let info = zpd_objc::msg_send!(zpd_objc::class!("NSMutableDictionary"), zpd_objc::sel!("dictionary"), () => Id);
            zpd_objc::msg_send!(info, zpd_objc::sel!("setObject:forKey:"), ((text.as_ptr()): Id, (NSAccessibilityAnnouncementKey): Id) => ());
            zpd_objc::msg_send!(info, zpd_objc::sel!("setObject:forKey:"), ((priority): Id, (NSAccessibilityPriorityKey): Id) => ());
            let app = zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => Id);
            NSAccessibilityPostNotificationWithUserInfo(
                app,
                NSAccessibilityAnnouncementRequestedNotification,
                info,
            );
        }
    }
}

/// Finds the cells that cell-based controls expose to VoiceOver instead of themselves.
struct Cells;

impl Cells {
    /// Returns the cell of `view`, or `None` when it has none.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    unsafe fn cell(view: Id) -> Option<Id> {
        // SAFETY: Guaranteed by the caller; `cell` is sent only when implemented.
        unsafe {
            if !zpd_objc::msg_send!(view, zpd_objc::sel!("respondsToSelector:"), ((zpd_objc::sel!("cell")): zpd_objc::Sel) => bool)
            {
                return None;
            }
            let cell = zpd_objc::msg_send!(view, zpd_objc::sel!("cell"), () => Id);
            (!cell.is_null()).then_some(cell)
        }
    }

    /// Returns the objects that receive attributes: the view and, for cell-based
    /// controls, the cell VoiceOver visits.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    unsafe fn targets(view: Id) -> [Option<Id>; 2] {
        // SAFETY: Guaranteed by the caller.
        [Some(view), unsafe { Self::cell(view) }]
    }

    /// Returns the object VoiceOver visits for `view`.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    unsafe fn element(view: Id) -> Id {
        // SAFETY: Guaranteed by the caller.
        unsafe { Self::cell(view) }.unwrap_or(view)
    }
}

impl ViewRef<'_> {
    /// Writes the attributes of `new` that differ from `old`.
    ///
    /// Pass the view's current attributes as `old`; for a new view that is
    /// [`AccessibilityAttributes`] with `element` set as the view starts. AppKit
    /// does not restore a view's own label when nil is written, so attributes
    /// that never changed are left alone.
    pub fn set_accessibility(
        self,
        old: &AccessibilityAttributes,
        new: &AccessibilityAttributes,
    ) -> Result<(), UiError> {
        let changed = |old: &Option<String>, new: &Option<String>| {
            (old != new).then(|| new.as_deref().map(native::nsstring))
        };
        let label = changed(&old.label, &new.label);
        let help = changed(&old.help, &new.help);
        let value = (old.value != new.value || old.replaces_value != new.replaces_value)
            .then(|| new.value.as_deref().map(native::nsstring));
        let role =
            (old.role != new.role).then(|| new.role.map(|role| native::nsstring(role.name())));
        let pointer = |value: &Option<Strong>| value.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr);
        self.with(|view| {
            // SAFETY: The live view and its cell implement the NSAccessibility protocol,
            // whose setters copy their NSString arguments.
            for view in unsafe { Cells::targets(view) }.into_iter().flatten() {
                unsafe {
                if old.element != new.element {
                    zpd_objc::msg_send!(view, zpd_objc::sel!("setAccessibilityElement:"), ((new.element): bool) => ());
                }
                if let Some(role) = &role {
                    zpd_objc::msg_send!(view, zpd_objc::sel!("setAccessibilityRole:"), ((pointer(role)): Id) => ());
                }
                if let Some(label) = &label {
                    zpd_objc::msg_send!(view, zpd_objc::sel!("setAccessibilityLabel:"), ((pointer(label)): Id) => ());
                }
                if let Some(help) = &help {
                    zpd_objc::msg_send!(view, zpd_objc::sel!("setAccessibilityHelp:"), ((pointer(help)): Id) => ());
                }
                if let Some(value) = &value {
                    if new.replaces_value {
                        zpd_objc::msg_send!(view, zpd_objc::sel!("setAccessibilityValue:"), ((pointer(value)): Id) => ());
                    } else {
                        zpd_objc::msg_send!(view, zpd_objc::sel!("setAccessibilityValueDescription:"), ((pointer(value)): Id) => ());
                    }
                }
                }
            }
        })
    }

    /// Names the view with the text of `title`; `None` removes the link.
    pub fn set_accessibility_title_element(
        self,
        title: Option<ViewRef<'_>>,
    ) -> Result<(), UiError> {
        // SAFETY: The title view is live; a cell-based label is read through its cell.
        let title = match title {
            Some(title) => title.with(|title| unsafe { Cells::element(title) })?,
            None => zpd_objc::NIL,
        };
        self.with(|view| {
            // SAFETY: The view, its cell, and the title are live; AppKit keeps the title weakly.
            for view in unsafe { Cells::targets(view) }.into_iter().flatten() {
                unsafe {
                    zpd_objc::msg_send!(view, zpd_objc::sel!("setAccessibilityTitleUIElement:"), ((title): Id) => ())
                }
            }
        })
    }

    /// Tells VoiceOver about `notification` on this view.
    pub fn post_accessibility_notification(
        self,
        notification: AccessibilityNotification,
    ) -> Result<(), UiError> {
        self.with(|view| {
            // SAFETY: The view is live and the names are AppKit NSString constants.
            unsafe {
                let name = match notification {
                    AccessibilityNotification::ValueChanged => {
                        NSAccessibilityValueChangedNotification
                    }
                    AccessibilityNotification::LayoutChanged => {
                        NSAccessibilityLayoutChangedNotification
                    }
                };
                NSAccessibilityPostNotification(view, name);
            }
        })
    }
}
