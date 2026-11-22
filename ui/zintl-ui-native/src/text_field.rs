use std::rc::Rc;

use zintl_ui::store::{Store, Trigger};
use zintl_ui_backend_core::graphics::{AttributedText, Font};
use zintl_ui_backend_core::{TextAlignment, TextColor, TextStyle, ViewKind};
use zintl_ui_layout::{LayoutDimension, LayoutStyle, Size};

use crate::modifiers;
use crate::{
    Change, ChangeRoute, Context, Element, Event, EventKind, NativeView, RenderNode, TextEstimate,
    ViewModifiers,
};

type CommitAction = dyn for<'a> Fn(&mut Context<'a>, &str);

/// An editable text field, or with [`crate::Text`] a static label.
///
/// It edits one line of plain text, or a few lines with
/// [`TextField::multiline`]. For document text that scrolls and keeps
/// formatting, use [`TextEditor`].
#[derive(Clone)]
pub struct TextField {
    value: String,
    binding: Option<Store<String>>,
    action: Option<Rc<Change<String>>>,
    commit: Option<Rc<CommitAction>>,
    placeholder: Option<String>,
    editable: bool,
    selectable: bool,
    bordered: bool,
    draws_background: bool,
    multiline: bool,
    style: TextStyle,
    focus: Option<Trigger>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
}

impl TextField {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            binding: None,
            action: None,
            commit: None,
            placeholder: None,
            editable: true,
            selectable: true,
            bordered: true,
            draws_background: true,
            multiline: false,
            style: TextStyle::default(),
            focus: None,
            layout: LayoutStyle::leaf(Size::new(160.0, 28.0)),
            modifiers: ViewModifiers::default(),
        }
    }

    /// Creates the static label behind [`crate::Text`].
    pub(crate) fn label(value: impl Into<String>) -> Self {
        let mut label = Self {
            value: value.into(),
            editable: false,
            selectable: false,
            bordered: false,
            draws_background: false,
            ..Self::new()
        };
        label.layout = LayoutStyle::leaf(Size::default());
        label.resize_label();
        label
    }

    /// Estimates a label's size from its text and style.
    ///
    /// Labels with a line limit prefer this size but can shrink to any width
    /// and truncate. Labels with a declared width and no limit wrap to it, and
    /// the backend measures their height. Other labels need their text's width.
    fn resize_label(&mut self) {
        let font_size = self
            .style
            .font
            .as_ref()
            .map_or(TextEstimate::SYSTEM_FONT_SIZE, |font| font.size() as f32);
        let lines = self.style.line_limit.unwrap_or(1).max(1) as f32;
        let height = (TextEstimate::line_height(font_size) * lines + 4.0)
            .ceil()
            .max(20.0);
        let width = TextEstimate::width(&self.value, font_size);
        let wraps = self.style.line_limit.is_none() && self.layout.width != LayoutDimension::Auto;
        let (minimum, content) = match self.style.line_limit {
            Some(_) => (Size::new(0.0, height), Some(Size::new(width, height))),
            None if wraps => (Size::new(0.0, height), None),
            None => (Size::new(width, height), None),
        };
        self.multiline = wraps;
        self.layout.wraps = wraps;
        self.layout.minimum_size = Some(minimum);
        self.layout.content_size = content;
    }

    /// Re-estimates a label's size after its style changes.
    fn restyle(mut self, update: impl FnOnce(&mut TextStyle)) -> Self {
        update(&mut self.style);
        if !self.editable {
            self.resize_label();
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

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn bind(mut self, store: Store<String>) -> Self {
        self.binding = Some(store);
        self
    }

    /// Runs after each edit, once the bound Store is updated.
    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static) -> Self {
        self.action = Some(ChangeRoute::text(action));
        self
    }

    /// Runs when editing ends with Return, Tab, or a focus change.
    ///
    /// Use it for work that should not follow each keystroke, such as renaming
    /// references or recording an undo step.
    pub fn on_commit(mut self, action: impl for<'a> Fn(&mut Context<'a>, &str) + 'static) -> Self {
        self.commit = Some(Rc::new(action));
        self
    }

    /// Wraps the text onto several lines with `true`.
    ///
    /// Use it for a few lines of plain text, such as a note on a form. The
    /// field grows with its text and does not scroll; for document text
    /// that scrolls and keeps formatting, use [`TextEditor`].
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    /// Takes keyboard focus each time `trigger` fires, for example after "New Item".
    ///
    /// A field created in the same update as the fire takes focus once it is shown.
    pub fn focus_on(mut self, trigger: Trigger) -> Self {
        self.focus = Some(trigger);
        self
    }
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}

native_view!(TextField);

impl NativeView for TextField {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let value = self
            .binding
            .map(|store| cx.get(store).clone())
            .unwrap_or_else(|| self.value.clone());
        // A label given a width wraps to it, so its size is estimated again.
        let mut sized = self.clone();
        if let (false, Some(width)) = (self.editable, self.modifiers.layout().width) {
            sized.layout.width = width;
            sized.resize_label();
        }
        let element = Element::node(RenderNode::View {
            kind: ViewKind::TextField {
                value,
                placeholder: self.placeholder.clone(),
                editable: self.editable,
                selectable: self.selectable,
                bordered: self.bordered,
                draws_background: self.draws_background,
                multiline: sized.multiline,
                style: self.style.clone(),
                focus: self.focus.map_or(0, |trigger| cx.revision(trigger.store())),
            },
            layout: sized.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        let element = ChangeRoute::new(self.binding, self.action.clone()).route(
            element,
            EventKind::TextChanged,
            |event| match event {
                Event::TextChanged { value } => Some(value),
                _ => None,
            },
        );
        let Some(commit) = self.commit.clone() else {
            return element;
        };
        element.on_event(EventKind::TextCommitted, move |cx, event| {
            if let Event::TextCommitted { value } = event {
                commit(cx, &value);
            }
        })
    }
}

/// A scrolling rich text editor that fills its parent, like a document body.
///
/// It edits styled runs and paragraph styles; `StandardAction::Bold` and the
/// paragraph style actions apply to it. For a few lines of plain text that
/// do not scroll, use [`TextField::multiline`]. Plain text converts with
/// `AttributedText::from(&str)` and [`AttributedText::text`].
#[derive(Clone)]
pub struct TextEditor {
    binding: Option<Store<AttributedText>>,
    action: Option<Rc<Change<AttributedText>>>,
    focus: Option<Trigger>,
    modifiers: ViewModifiers,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            binding: None,
            action: None,
            focus: None,
            modifiers: ViewModifiers::default(),
        }
    }

    /// Reads and writes the edited text.
    pub fn bind(mut self, store: Store<AttributedText>) -> Self {
        self.binding = Some(store);
        self
    }

    /// Runs after each edit, once the bound Store is updated.
    pub fn on_change(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, &AttributedText) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Takes keyboard focus each time `trigger` fires, for example after creating a note.
    pub fn focus_on(mut self, trigger: Trigger) -> Self {
        self.focus = Some(trigger);
        self
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeView for TextEditor {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let element = Element::node(RenderNode::View {
            kind: ViewKind::TextEditor {
                text: self
                    .binding
                    .map_or_else(AttributedText::new, |store| cx.get(store).clone()),
                focus: self.focus.map_or(0, |trigger| cx.revision(trigger.store())),
            },
            layout: LayoutStyle::fill(Size::new(80.0, 40.0)),
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        ChangeRoute::new(self.binding, self.action.clone()).route(
            element,
            EventKind::RichTextChanged,
            |event| match event {
                Event::RichTextChanged { text } => Some(text),
                _ => None,
            },
        )
    }
}

native_view!(TextEditor);
