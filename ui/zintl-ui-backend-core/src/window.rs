//! Placement of declared windows on screen.

use zintl_ui_layout::Size;

use crate::Point;

/// Where a synchronization moves a window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowPosition {
    /// The frame's top-left corner, in screen points with a top-left origin.
    At(Point),
    /// The middle of the window's screen.
    Center,
}

/// The size and position one synchronization applies to a window.
///
/// Unchanged declarations are not applied again, so a window the user moved
/// or resized stays where they left it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowPlacement {
    /// The content size to apply.
    pub size: Option<Size>,
    pub position: Option<WindowPosition>,
}

impl WindowPlacement {
    /// Compares the declared size and position with the ones last applied.
    ///
    /// A new window takes both, centered without a position. Later, the
    /// content size is applied again when it changes or `refit` asks for it,
    /// such as after a title bar change on macOS, and a position only when it
    /// changes. Applying a size keeps the frame's top-left corner.
    pub fn changes(
        new_window: bool,
        refit: bool,
        (applied_size, applied_position): (Option<Size>, Option<Point>),
        (size, position): (Size, Option<Point>),
    ) -> Self {
        let resize = new_window || refit || applied_size != Some(size);
        let position = match position {
            Some(position) if new_window || applied_position != Some(position) => {
                Some(WindowPosition::At(position))
            }
            None if new_window => Some(WindowPosition::Center),
            _ => None,
        };
        Self {
            size: resize.then_some(size),
            position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_windows_take_their_size_and_center_without_a_position() {
        // Verifies a new window gets its content size, and its position or else the screen's center.
        let size = Size::new(640.0, 480.0);
        let corner = Point::new(10.0, 20.0);

        assert_eq!(
            WindowPlacement::changes(true, false, (None, None), (size, None)),
            WindowPlacement {
                size: Some(size),
                position: Some(WindowPosition::Center),
            }
        );
        assert_eq!(
            WindowPlacement::changes(true, false, (None, None), (size, Some(corner))).position,
            Some(WindowPosition::At(corner))
        );
    }

    #[test]
    fn unchanged_window_declarations_are_not_reapplied() {
        // Verifies later synchronization keeps a window the user moved or resized, and applies only what changed.
        let size = Size::new(640.0, 480.0);
        let corner = Point::new(10.0, 20.0);
        let applied = (Some(size), Some(corner));

        let unchanged = WindowPlacement::changes(false, false, applied, (size, Some(corner)));
        assert_eq!(
            unchanged,
            WindowPlacement {
                size: None,
                position: None
            }
        );

        // A new size resizes without moving.
        let resized = WindowPlacement::changes(
            false,
            false,
            applied,
            (Size::new(800.0, 600.0), Some(corner)),
        );
        assert_eq!(resized.size, Some(Size::new(800.0, 600.0)));
        assert_eq!(resized.position, None);

        // A new position moves without resizing.
        let moved = Point::new(30.0, 40.0);
        let relocated = WindowPlacement::changes(false, false, applied, (size, Some(moved)));
        assert_eq!(
            relocated,
            WindowPlacement {
                size: None,
                position: Some(WindowPosition::At(moved))
            }
        );

        // Dropping the position leaves the window where it is.
        assert_eq!(
            WindowPlacement::changes(false, false, applied, (size, None)).position,
            None
        );

        // A title bar change re-applies the content size, which its frame depends on.
        assert_eq!(
            WindowPlacement::changes(false, true, applied, (size, Some(corner))).size,
            Some(size)
        );
    }
}
