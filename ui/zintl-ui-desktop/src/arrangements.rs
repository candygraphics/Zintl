//! Containers that overlay, tabulate, or fold their children.

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::view::View;
use zintl_ui_layout::{Arrangement, Axis, GridCell, LayoutDimension, LayoutStyle, Size};

use crate::{
    AccessibilityRole, Attach, Attachments, Canvas2D, Canvas2DHandler, Color, Context, Element,
    Frame, Graphics2D, ListFactory, NativeView, RenderNode, Store, Text, list,
};

/// A plain native container for the arrangements below.
struct Container;

impl Container {
    /// Renders `children` laid out by `layout`.
    fn element(
        layout: LayoutStyle,
        id: &Option<String>,
        children: ListFactory<RenderNode>,
        cx: &mut Context<'_>,
    ) -> Element<RenderNode> {
        let view = zintl_ui_native::View::new(layout, children);
        match id {
            Some(id) => view.id(id.clone()).native_element(cx),
            None => view.native_element(cx),
        }
    }
}

/// Stacks its children on top of each other, later children in front.
///
/// The largest child sets the size, and every child stretches to it, like a
/// label over a 3D preview.
#[derive(Clone)]
pub struct ZStack {
    children: ListFactory<RenderNode>,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl ZStack {
    pub fn new(children: ListFactory<RenderNode>) -> Self {
        let mut layout = LayoutStyle::stack(Axis::Vertical, 0.0);
        layout.arrangement = Arrangement::Overlay;
        Self {
            children,
            layout,
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.layout.width = LayoutDimension::Percent(1.0);
        self
    }

    /// Takes the remaining space along the parent's main axis.
    pub fn grow(mut self) -> Self {
        self.layout.grow = 1.0;
        self
    }
}

impl NativeView for ZStack {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        Container::element(self.layout, &self.id, self.children.clone(), cx)
    }

    fn attachments(&self) -> &Attachments {
        &self.attachments
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        &mut self.attachments
    }
}

/// Fills its frame with one color, behind a grid row.
#[derive(Clone)]
struct Stripe;

impl Canvas2DHandler for Stripe {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        g.clear(Color::TRANSPARENT);
        g.fill_rect(frame.bounds, Color::rgba(0.5, 0.5, 0.5, 0.1));
    }
}

/// Lays out children row by row in aligned columns, like a table without headers.
///
/// Each child is one cell; each column is as wide as its widest cell.
#[derive(Clone)]
pub struct Grid {
    columns: u16,
    children: ListFactory<RenderNode>,
    striped: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl Grid {
    pub fn new(columns: u16, children: ListFactory<RenderNode>) -> Self {
        let mut layout = LayoutStyle::stack(Axis::Vertical, 4.0);
        layout.arrangement = Arrangement::Grid {
            columns: columns.max(1),
        };
        Self {
            columns: columns.max(1),
            children,
            striped: false,
            layout,
            id: None,
            attachments: Attachments::default(),
        }
    }

    /// Shades every other row.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Sets the gap between rows and between columns.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.layout.gap = spacing;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.layout.width = LayoutDimension::Percent(1.0);
        self
    }
}

impl NativeView for Grid {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let rows = self.children.len().div_ceil(self.columns as usize);
        // Stripes come first so the cells draw over them.
        let mut children: ListFactory<RenderNode> = Vec::new();
        if self.striped {
            for row in (1..rows).step_by(2) {
                let mut layout = LayoutStyle::stack(Axis::Horizontal, 0.0);
                layout.grid_cell = Some(GridCell {
                    row: row as u16,
                    column: 0,
                    column_span: 0,
                });
                children.push(ElementFactory::new(zintl_ui_native::View::new(
                    layout,
                    list![Canvas2D::new(Stripe).grow()],
                )));
            }
        }
        children.extend(self.children.iter().cloned());
        Container::element(self.layout, &self.id, children, cx)
    }

    fn attachments(&self) -> &Attachments {
        &self.attachments
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        &mut self.attachments
    }
}

/// A titled section whose content shows only while it is expanded.
#[derive(Clone)]
pub struct DisclosureGroup {
    title: String,
    content: ElementFactory<RenderNode>,
    binding: Option<Store<bool>>,
    expanded: Store<bool>,
    id: Option<String>,
    attachments: Attachments,
}

impl DisclosureGroup {
    pub fn new<V>(title: impl Into<String>, content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            title: title.into(),
            content: ElementFactory::new(content),
            binding: None,
            expanded: Store::default(),
            id: None,
            attachments: Attachments::default(),
        }
    }

    /// Reads and writes whether the group is expanded; it starts collapsed otherwise.
    pub fn bind(mut self, expanded: Store<bool>) -> Self {
        self.binding = Some(expanded);
        self
    }

    /// Names the disclosure button, such as for UI tests.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl View for DisclosureGroup {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.expanded = self.binding.unwrap_or_else(|| cx.store(false));
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        self.native_element(cx)
    }
}

impl NativeView for DisclosureGroup {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let expanded = self.expanded;
        let open = *cx.get(expanded);
        // The triangle carries the title for VoiceOver, so the visible title is not read twice.
        let mut toggle = zintl_ui_native::Button::new("")
            .bezel_style(Some(zintl_ui_native::BezelStyle::Disclosure))
            .button_type(zintl_ui_native::ButtonType::PushOnPushOff)
            .minimum_size(Size::new(13.0, 13.0))
            .bind_state(expanded)
            .accessibility_label(self.title.clone());
        if let Some(id) = &self.id {
            toggle = toggle.id(id.clone());
        }
        let title = Text::new(self.title.clone()).accessibility_hidden();
        let header = crate::HStack::new(list![toggle, title]).spacing(4.0);
        let mut rows = list![header];
        if open {
            rows.push(self.content.clone());
        }
        crate::VStack::new(rows)
            .spacing(6.0)
            .accessibility_label(self.title.clone())
            .accessibility_role(AccessibilityRole::Group)
            .native_element(cx)
    }

    fn attachments(&self) -> &Attachments {
        &self.attachments
    }

    fn attachments_mut(&mut self) -> &mut Attachments {
        &mut self.attachments
    }
}
