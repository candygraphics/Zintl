use std::rc::Rc;

use zintl_ui::element::IntoElement;
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_backend_core::graphics::Font;
use zintl_ui_backend_core::{TextAlignment, TextColor, TextStyle, ViewKind};
use zintl_ui_layout::{LayoutDimension, LayoutStyle, Size};

use crate::attach::attachments;
use crate::{Attachments, Context, Element, Event, EventKind, NativeView, RenderNode};

type ChangeAction = dyn for<'a> Fn(&mut Context<'a>, &str);

#[derive(Clone)]
pub struct TextField {
    value: String,
    binding: Option<Store<String>>,
    action: Option<Rc<ChangeAction>>,
    placeholder: Option<String>,
    editable: bool,
    selectable: bool,
    bordered: bool,
    draws_background: bool,
    multiline: bool,
    style: TextStyle,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl TextField {
    pub fn new() -> Self {
        Self::with_string("")
    }

    pub fn with_string(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            binding: None,
            action: None,
            placeholder: None,
            editable: true,
            selectable: true,
            bordered: true,
            draws_background: true,
            multiline: false,
            style: TextStyle::default(),
            layout: LayoutStyle::leaf(Size::new(160.0, 28.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn label_with_string(value: impl Into<String>) -> Self {
        let mut label = Self {
            editable: false,
            selectable: false,
            bordered: false,
            draws_background: false,
            ..Self::with_string(value)
        };
        label.layout = LayoutStyle::leaf(label.label_size());
        label
    }

    /// Estimates a label's size from its text and style.
    ///
    /// Labels with a line limit can shrink to any width and truncate.
    fn label_size(&self) -> Size {
        const STANDARD_SIZE: f64 = 13.0;
        let font_size = self.style.font.as_ref().map_or(STANDARD_SIZE, Font::size) as f32;
        let lines = self.style.line_limit.unwrap_or(1).max(1) as f32;
        let height = (font_size * 1.25 * lines + 4.0).ceil().max(20.0);
        let width = match self.style.line_limit {
            Some(_) => 0.0,
            None => self.value.chars().count() as f32 * font_size * 0.54,
        };
        Size::new(width, height)
    }

    /// Re-estimates a label's size after its style changes.
    fn restyle(mut self, update: impl FnOnce(&mut TextStyle)) -> Self {
        update(&mut self.style);
        if !self.editable {
            self.layout.minimum_size = self.label_size();
        }
        self
    }

    pub fn font(self, font: Font) -> Self {
        self.restyle(|style| style.font = Some(font))
    }

    pub fn color(self, color: TextColor) -> Self {
        self.restyle(|style| style.color = color)
    }

    pub fn alignment(self, alignment: TextAlignment) -> Self {
        self.restyle(|style| style.alignment = alignment)
    }

    /// Shows at most `lines` lines and truncates the rest with an ellipsis.
    pub fn line_limit(self, lines: usize) -> Self {
        self.restyle(|style| style.line_limit = Some(lines))
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn bind(mut self, store: Store<String>) -> Self {
        self.binding = Some(store);
        self
    }

    /// Runs after each edit, once the bound Store is updated.
    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Takes the parent's full width.
    pub fn fill_width(mut self) -> Self {
        self.layout.width = LayoutDimension::Percent(1.0);
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.layout.minimum_size = size;
        self
    }
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}

impl View for TextField {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}

impl NativeView for TextField {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let value = self
            .binding
            .map(|store| cx.get(store).clone())
            .unwrap_or_else(|| self.value.clone());
        let element = Element::node(RenderNode::View {
            kind: ViewKind::TextField {
                value,
                placeholder: self.placeholder.clone(),
                editable: self.editable,
                selectable: self.selectable,
                bordered: self.bordered,
                draws_background: self.draws_background,
                multiline: self.multiline,
                style: self.style.clone(),
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: true,
            title_bar_interactive: false,
        });
        TextChange::route(element, self.binding, self.action.clone())
    }
}

/// Routes text edits to a bound Store and a change action.
pub(crate) struct TextChange;

impl TextChange {
    pub(crate) fn route(
        element: Element<RenderNode>,
        binding: Option<Store<String>>,
        action: Option<Rc<ChangeAction>>,
    ) -> Element<RenderNode> {
        if binding.is_none() && action.is_none() {
            return element;
        }
        element.on_event(EventKind::TextChanged, move |cx, event| {
            if let Event::TextChanged { value } = event {
                if let Some(store) = binding {
                    cx.update(store, |current| current.clone_from(&value));
                }
                if let Some(action) = &action {
                    action(cx, &value);
                }
            }
        })
    }
}

/// A scrolling multiline editor that fills its parent, like a document body.
#[derive(Clone)]
pub struct TextEditor {
    binding: Option<Store<String>>,
    action: Option<Rc<ChangeAction>>,
    font: Option<Font>,
    editable: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            binding: None,
            action: None,
            font: None,
            editable: true,
            layout: LayoutStyle::fill(Size::new(80.0, 40.0)),
            id: None,
            attachments: Attachments::default(),
        }
    }

    pub fn bind(mut self, store: Store<String>) -> Self {
        self.binding = Some(store);
        self
    }

    /// Runs after each edit, once the bound Store is updated.
    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.layout.minimum_size = size;
        self
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for TextEditor {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let value = self
            .binding
            .map(|store| cx.get(store).clone())
            .unwrap_or_default();
        let element = Element::node(RenderNode::View {
            kind: ViewKind::TextEditor {
                value,
                font: self.font.clone(),
                editable: self.editable,
            },
            layout: self.layout,
            id: self.id.clone(),
            enabled: true,
            title_bar_interactive: false,
        });
        TextChange::route(element, self.binding, self.action.clone())
    }
}

impl View for TextEditor {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
