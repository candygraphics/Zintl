//! What assistive apps such as VoiceOver report for views, and spoken announcements.

use std::cell::RefCell;

/// What assistive apps report for one view, on top of what the platform infers.
///
/// Fields left as `None` keep the platform's own description, such as a
/// button's title.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Accessibility {
    /// Names the view, replacing its title or text.
    pub label: Option<String>,
    /// Describes the current value, such as "50 percent".
    pub value: Option<String>,
    /// Explains what using the view does.
    pub hint: Option<String>,
    /// Replaces the role the platform reports.
    pub role: Option<AccessibilityRole>,
    /// Hides the view itself; its subviews stay reachable.
    pub hidden: bool,
    /// The ID of the view whose text names this view.
    pub labeled_by: Option<String>,
}

impl Accessibility {
    /// Returns whether nothing overrides the platform's description.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Replaces the fields that `overrides` sets.
    pub fn merge(&mut self, overrides: &Self) {
        let Self {
            label,
            value,
            hint,
            role,
            hidden,
            labeled_by,
        } = overrides;
        if label.is_some() {
            self.label.clone_from(label);
        }
        if value.is_some() {
            self.value.clone_from(value);
        }
        if hint.is_some() {
            self.hint.clone_from(hint);
        }
        if role.is_some() {
            self.role = *role;
        }
        self.hidden |= hidden;
        if labeled_by.is_some() {
            self.labeled_by.clone_from(labeled_by);
        }
    }

    /// Returns whether a container or canvas becomes an element of its own.
    ///
    /// Such views are skipped by default so assistive apps reach their content
    /// directly; a label or role makes them a named element.
    pub fn exposes_container(&self) -> bool {
        !self.hidden && (self.label.is_some() || self.role.is_some())
    }
}

/// A role that views without a fitting native control can report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
    /// Related views read as one named section.
    Group,
    /// A picture, such as a drawn chart.
    Image,
    /// Text that cannot be edited.
    StaticText,
}

/// How urgently an [`Announcement`] interrupts speech.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnnouncementPriority {
    /// Waits until other speech ends, and may be dropped.
    Low,
    /// Waits until other speech ends.
    #[default]
    Medium,
    /// Interrupts other speech.
    High,
}

/// Text that assistive apps speak without moving focus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Announcement {
    pub text: String,
    pub priority: AnnouncementPriority,
}

thread_local! {
    static ANNOUNCEMENTS: RefCell<Vec<Announcement>> = const { RefCell::new(Vec::new()) };
}

/// The announcements queued on this thread by [`crate::Context::announce`].
///
/// Backends post them after rendering, so they follow the changes that
/// caused them.
pub struct Announcements;

impl Announcements {
    pub(crate) fn push(announcement: Announcement) {
        ANNOUNCEMENTS.with(|queue| queue.borrow_mut().push(announcement));
    }

    /// Removes the queued announcements, oldest first.
    pub fn take() -> Vec<Announcement> {
        ANNOUNCEMENTS.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_keeps_unset_fields() {
        // Verifies overrides replace only the fields they set.
        let mut base = Accessibility {
            label: Some("Base".into()),
            hint: Some("Base hint".into()),
            ..Accessibility::default()
        };
        base.merge(&Accessibility {
            label: Some("Override".into()),
            hidden: true,
            ..Accessibility::default()
        });

        assert_eq!(base.label.as_deref(), Some("Override"));
        assert_eq!(base.hint.as_deref(), Some("Base hint"));
        assert!(base.hidden);
    }

    #[test]
    fn containers_need_a_label_or_role() {
        // Verifies containers become elements only when named and not hidden.
        let named = Accessibility {
            label: Some("Toolbox".into()),
            ..Accessibility::default()
        };
        let hidden = Accessibility {
            hidden: true,
            ..named.clone()
        };

        assert!(!Accessibility::default().exposes_container());
        assert!(named.exposes_container());
        assert!(!hidden.exposes_container());
    }

    #[test]
    fn announcements_drain_in_order() {
        // Verifies the queue returns announcements oldest first and then empties.
        Announcements::take();
        for text in ["first", "second"] {
            Announcements::push(Announcement {
                text: text.into(),
                priority: AnnouncementPriority::Medium,
            });
        }

        let texts: Vec<_> = Announcements::take().into_iter().map(|a| a.text).collect();
        assert_eq!(texts, ["first", "second"]);
        assert!(Announcements::take().is_empty());
    }
}
