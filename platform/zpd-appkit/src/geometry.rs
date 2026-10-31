/// Rectangle in logical points / DIP.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

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

/// A point in logical points / DIP.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl From<Point> for crate::native::Point {
    fn from(point: Point) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<crate::native::Point> for Point {
    fn from(point: crate::native::Point) -> Self {
        Self::new(point.x, point.y)
    }
}
