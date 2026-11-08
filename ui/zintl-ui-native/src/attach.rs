use zintl_ui_backend_core::{Accessibility, AccessibilityRole};

use crate::{Context, Element, Menu, Popover, RenderNode};

/// A view that renders exactly one native element.
///
/// Built-in views implement this so presentation modifiers can attach to
/// the native view they render.
pub trait NativeView: Clone + 'static {
    /// Renders the native element without its attachments.
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode>;

    fn attachments(&self) -> &Attachments;

    fn attachments_mut(&mut self) -> &mut Attachments;

    /// Renders the native element with its context menu, popovers, and input options.
    fn native_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let mut element = self.base_element(cx);
        if let Element::Node {
            value:
                RenderNode::View {
                    title_bar_interactive,
                    help,
                    accessibility,
                    ..
                },
            ..
        } = &mut element
        {
            *title_bar_interactive = self.attachments().title_bar_interactive;
            help.clone_from(&self.attachments().help);
            // Views may describe themselves; modifiers win over that.
            accessibility.merge(&self.attachments().accessibility);
        }
        self.attachments().attach(element, cx)
    }
}

/// Presentation modifiers available on every [`NativeView`].
///
/// They return `Self`, so the view's own builder methods can follow.
pub trait Attach: NativeView + Sized {
    /// Shows `menu` when the view is secondary-clicked.
    fn context_menu(mut self, menu: impl Into<Menu>) -> Self {
        self.attachments_mut().menu = Some(menu.into());
        self
    }

    /// Anchors `popover` to this view.
    fn popover(mut self, popover: Popover) -> Self {
        self.attachments_mut().popovers.push(popover);
        self
    }

    /// Keeps mouse input for this view and its subviews inside the title bar area.
    ///
    /// When a window's content extends under its title bar, clicks there move
    /// the window unless they land on a view marked with this modifier.
    fn title_bar_interactive(mut self) -> Self {
        self.attachments_mut().title_bar_interactive = true;
        self
    }

    /// Shows `text` when the pointer rests on the view (`NSView.toolTip`).
    ///
    /// It also shows while the view is disabled, so it can say why.
    fn help(mut self, text: impl Into<String>) -> Self {
        self.attachments_mut().help = Some(text.into());
        self
    }

    /// Names the view for assistive apps such as VoiceOver.
    ///
    /// Use it for controls without visible text, such as a button showing only
    /// a symbol. A label also turns a container or canvas into a named element.
    fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.attachments_mut().accessibility.label = Some(label.into());
        self
    }

    /// Describes the view's current value, such as "3 of 10 done".
    fn accessibility_value(mut self, value: impl Into<String>) -> Self {
        self.attachments_mut().accessibility.value = Some(value.into());
        self
    }

    /// Explains what using the view does, such as "Removes the selected train".
    fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.attachments_mut().accessibility.hint = Some(hint.into());
        self
    }

    /// Replaces the role assistive apps report, such as [`AccessibilityRole::Group`]
    /// for a stack that reads as one section.
    fn accessibility_role(mut self, role: AccessibilityRole) -> Self {
        self.attachments_mut().accessibility.role = Some(role);
        self
    }

    /// Hides the view itself from assistive apps, such as decorative text that
    /// repeats a neighbor; its subviews stay reachable.
    fn accessibility_hidden(mut self) -> Self {
        self.attachments_mut().accessibility.hidden = true;
        self
    }

    /// Names the view with the text of the view whose ID is `id`, such as a
    /// `Text` beside a slider.
    fn accessibility_labeled_by(mut self, id: impl Into<String>) -> Self {
        self.attachments_mut().accessibility.labeled_by = Some(id.into());
        self
    }
}

impl<V: NativeView> Attach for V {}

/// The context menu and popovers attached to a native view.
#[derive(Clone, Default)]
pub struct Attachments {
    menu: Option<Menu>,
    popovers: Vec<Popover>,
    title_bar_interactive: bool,
    help: Option<String>,
    accessibility: Accessibility,
}

impl Attachments {
    fn attach(&self, element: Element<RenderNode>, cx: &mut Context<'_>) -> Element<RenderNode> {
        if self.menu.is_none() && self.popovers.is_empty() {
            return element;
        }
        let mut attachments = Vec::new();
        if let Some(menu) = &self.menu {
            attachments.push(menu.context_element());
        }
        attachments.extend(self.popovers.iter().map(|popover| popover.element(cx)));
        element.with_children(attachments)
    }
}

/// Implements the [`NativeView`] attachment accessors for an `attachments` field.
macro_rules! attachments {
    () => {
        fn attachments(&self) -> &$crate::Attachments {
            &self.attachments
        }

        fn attachments_mut(&mut self) -> &mut $crate::Attachments {
            &mut self.attachments
        }
    };
}

pub(crate) use attachments;
