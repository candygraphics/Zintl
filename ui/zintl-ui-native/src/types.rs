//! View configuration values that only the declarative layer uses.

/// The semantic role of a button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonRole {
    #[default]
    Normal,
    /// Answers Return and draws with the accent color.
    Default,
    /// Answers Escape.
    Cancel,
    /// Marks an action that destroys data.
    Destructive,
}

/// The saved state of a scroll view, bound with [`crate::ScrollView::bind`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollStatus {
    /// The horizontal document offset shown at the leading edge.
    pub x: f64,
    /// The vertical document offset shown at the top edge.
    pub y: f64,
}
