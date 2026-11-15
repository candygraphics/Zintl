use zintl_ui::element::{IntoElement, ListFactory};
use zintl_ui::view::View as ViewTrait;
use zintl_ui_backend_core::ViewKind;
use zintl_ui_layout::LayoutStyle;

use crate::modifiers;
use crate::{Context, Element, NativeView, RenderNode, ViewModifiers};

#[derive(Clone)]
pub struct View {
    children: ListFactory<RenderNode>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl View {
    pub fn new(layout: LayoutStyle, children: ListFactory<RenderNode>) -> Self {
        Self {
            children,
            layout,
            modifiers: ViewModifiers::default(),
        }
    }
}

impl NativeView for View {
    modifiers!();

    fn base_element(&self, _cx: &mut Context<'_>) -> Element<RenderNode> {
        Element::node(RenderNode::View {
            kind: ViewKind::Container,
            layout: self.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        })
        .with_children([self.children.clone().into_element()])
    }
}

impl ViewTrait for View {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
