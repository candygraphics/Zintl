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

    /// Renders the native element with its context menu and popovers.
    fn native_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = self.base_element(cx);
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
}

impl<V: NativeView> Attach for V {}

/// The context menu and popovers attached to a native view.
#[derive(Clone, Default)]
pub struct Attachments {
    menu: Option<Menu>,
    popovers: Vec<Popover>,
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
