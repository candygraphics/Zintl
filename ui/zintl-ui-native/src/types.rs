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

/// The name of a view, used by accessibility, UI tests, and scroll targets.
///
/// String literals convert into it, so `.id("save-button")` works directly.
/// Share an ID between views and tests as a `&'static str` constant.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViewId(String);

impl ViewId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Lets views derive IDs for their parts, such as `format!("{id}-clear")`.
impl std::fmt::Display for ViewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ViewId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ViewId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&ViewId> for ViewId {
    fn from(value: &ViewId) -> Self {
        value.clone()
    }
}

impl From<ViewId> for String {
    fn from(value: ViewId) -> Self {
        value.0
    }
}
