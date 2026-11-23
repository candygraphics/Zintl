//! Static text.

use zintl_ui_backend_core::graphics::Font;
use zintl_ui_backend_core::{TextAlignment, TextColor};

use crate::{Context, Element, NativeView, RenderNode, TextField, ViewModifiers};

/// Text the user reads but does not edit.
#[derive(Clone)]
pub struct Text {
    label: TextField,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            label: TextField::label(content),
        }
    }

    pub fn font(mut self, font: Font) -> Self {
        self.label = self.label.font(font);
        self
    }

    pub fn color(mut self, color: TextColor) -> Self {
        self.label = self.label.color(color);
        self
    }

    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.label = self.label.alignment(alignment);
        self
    }

    /// Lets the user select the text and copy it with `true`; it still looks like plain text.
    ///
    /// In a table, the first click selects the row and a click on the
    /// selected row's text selects the text.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.label = self.label.selectable(selectable);
        self
    }

    /// Shows at most `lines` lines and truncates the rest with an ellipsis.
    ///
    /// A limited label no longer reserves its text's width, so give it one
    /// with `fill_width` or a stretching parent.
    pub fn line_limit(mut self, lines: usize) -> Self {
        self.label = self.label.line_limit(lines);
        self
    }
}

impl NativeView for Text {
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        self.label.base_element(cx)
    }

    fn modifiers(&self) -> &ViewModifiers {
        self.label.modifiers()
    }

    fn modifiers_mut(&mut self) -> &mut ViewModifiers {
        self.label.modifiers_mut()
    }
}

native_view!(Text);
