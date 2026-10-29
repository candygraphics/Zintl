pub use crate::model::Rect;

impl From<Rect> for crate::native::Rect {
    fn from(rect: Rect) -> Self {
        Self {
            origin: crate::native::Point {
                x: rect.x,
                y: rect.y,
            },
            size: crate::native::Size {
                width: rect.width,
                height: rect.height,
            },
        }
    }
}

impl From<crate::native::Rect> for Rect {
    fn from(rect: crate::native::Rect) -> Self {
        Self::new(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        )
    }
}

/// Size in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

impl PhysicalSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}
