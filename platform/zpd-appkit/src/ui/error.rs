use crate::actor::ActorError;

/// Error returned by AppKit window, view, sidebar, and command operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiError {
    NativeCreationFailed,
    Closed,
    InvalidHierarchy,
}
impl std::fmt::Display for UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NativeCreationFailed => "AppKit failed to create a native object",
            Self::Closed => "the native object is closed",
            Self::InvalidHierarchy => "the requested native hierarchy is invalid",
        })
    }
}
impl std::error::Error for UiError {}
impl From<ActorError> for UiError {
    fn from(value: ActorError) -> Self {
        match value {
            ActorError::InvalidHierarchy => Self::InvalidHierarchy,
            ActorError::Dropped | ActorError::NativeReleased => Self::Closed,
        }
    }
}
