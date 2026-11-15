use std::rc::Rc;

use zintl_ui::store::Store;
use zintl_ui_backend_core::graphics::{AttributedText, Font};
use zintl_ui_backend_core::{TextAlignment, TextColor, TextStyle, ViewKind};
use zintl_ui_layout::{LayoutDimension, LayoutStyle, Size};

use crate::modifiers;
use crate::{Context, Element, Event, EventKind, NativeView, RenderNode, ViewModifiers};

type ChangeAction = dyn for<'a> Fn(&mut Context<'a>, &str);

#[derive(Clone)]
pub struct TextField {
    value: String,
    binding: Option<Store<String>>,
    action: Option<Rc<ChangeAction>>,
    commit: Option<Rc<ChangeAction>>,
    placeholder: Option<String>,
    editable: bool,
    selectable: bool,
    bordered: bool,
    draws_background: bool,
    multiline: bool,
    style: TextStyle,
    focus: Option<Store<u64>>,
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
        const STANDARD_SIZE: f64 = 13.0;
        let font_size = self.style.font.as_ref().map_or(STANDARD_SIZE, Font::size) as f32;
        let lines = self.style.line_limit.unwrap_or(1).max(1) as f32;
        let height = (font_size * 1.25 * lines + 4.0).ceil().max(20.0);
        let width = self.value.chars().count() as f32 * font_size * 0.54;
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

    /// Shows `value`; a bound Store takes precedence.
    ///
    /// The field keeps typed text until `value` itself changes.
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
        self.action = Some(Rc::new(action));
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

    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Takes keyboard focus whenever `requests` increases, for example after "New Item".
    ///
    /// A field created in the same update as the request takes focus once it is shown.
    pub fn focus_on(mut self, requests: Store<u64>) -> Self {
        self.focus = Some(requests);
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
                focus: self.focus.map_or(0, |store| *cx.get(store)),
            },
            layout: sized.layout,
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        let element = TextChange::route(element, self.binding, self.action.clone());
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

/// Routes text edits to a bound Store and a change action.
struct TextChange;

impl TextChange {
    fn route(
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

type EditAction = dyn for<'a> Fn(&mut Context<'a>);

/// A scrolling rich text editor that fills its parent, like a document body.
///
/// It edits styled runs and paragraph styles; `StandardAction::Bold` and the
/// paragraph style actions apply to it.
#[derive(Clone)]
pub struct TextEditor {
    binding: Option<Store<AttributedText>>,
    action: Option<Rc<EditAction>>,
    focus: Option<Store<u64>>,
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
    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Takes keyboard focus whenever `requests` increases, for example after creating a note.
    pub fn focus_on(mut self, requests: Store<u64>) -> Self {
        self.focus = Some(requests);
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
                focus: self.focus.map_or(0, |store| *cx.get(store)),
            },
            layout: LayoutStyle::fill(Size::new(80.0, 40.0)),
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        let (store, action) = (self.binding, self.action.clone());
        if store.is_none() && action.is_none() {
            return element;
        }
        element.on_event(EventKind::RichTextChanged, move |cx, event| {
            if let Event::RichTextChanged { text } = event {
                if let Some(store) = store {
                    cx.update(store, |current| current.clone_from(&text));
                }
                if let Some(action) = &action {
                    action(cx);
                }
            }
        })
    }
}

native_view!(TextEditor);
