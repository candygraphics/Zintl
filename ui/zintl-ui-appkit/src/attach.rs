use zintl_ui::element::IntoElement;
use zintl_ui::view::View;

use crate::{Context, Element, Menu, Popover, RenderNode};

/// A view that renders exactly one native AppKit element.
///
/// Built-in views implement this so presentation modifiers can attach to
/// the native view they render.
pub trait NativeView: Clone + 'static {
    fn native_element(&self, cx: &mut Context<'_>) -> Element<RenderNode>;
}

/// Presentation modifiers available on every [`NativeView`].
pub trait Attach: NativeView + Sized {
    /// Shows `menu` when the view is secondary-clicked.
    fn context_menu(self, menu: impl Into<Menu>) -> Attached<Self> {
        Attached::new(self).context_menu(menu)
    }

    /// Anchors `popover` to this view.
    fn popover(self, popover: Popover) -> Attached<Self> {
        Attached::new(self).popover(popover)
    }
}

impl<V: NativeView> Attach for V {}

/// A native view with a context menu or popovers attached.
#[derive(Clone)]
pub struct Attached<V> {
    view: V,
    menu: Option<Menu>,
    popovers: Vec<Popover>,
}

impl<V: NativeView> Attached<V> {
    fn new(view: V) -> Self {
        Self {
            view,
            menu: None,
            popovers: Vec::new(),
        }
    }

    pub fn context_menu(mut self, menu: impl Into<Menu>) -> Self {
        self.menu = Some(menu.into());
        self
    }

    pub fn popover(mut self, popover: Popover) -> Self {
        self.popovers.push(popover);
        self
    }
}

impl<V: NativeView> NativeView for Attached<V> {
    fn native_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let mut attachments = Vec::new();
        if let Some(menu) = &self.menu {
            attachments.push(menu.context_element());
        }
        attachments.extend(self.popovers.iter().map(|popover| popover.element(cx)));
        self.view.native_element(cx).with_children(attachments)
    }
}

impl<V: NativeView> View for Attached<V> {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
