use crate::{Color, Point};

/// A color at `offset` in `0.0..=1.0` along a gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}

impl GradientStop {
    pub const fn new(offset: f64, color: Color) -> Self {
        Self { offset, color }
    }
}

/// A gradient along the line from `start` to `end`, extended past both ends.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    pub start: Point,
    pub end: Point,
    pub stops: Vec<GradientStop>,
}

/// A gradient from `center` outward to `radius`, extended past the edge.
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradient {
    pub center: Point,
    pub radius: f64,
    pub stops: Vec<GradientStop>,
}

/// What fills or strokes a shape.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Solid(Color),
    Linear(LinearGradient),
    Radial(RadialGradient),
}

impl Paint {
    pub fn linear(start: Point, end: Point, stops: impl Into<Vec<GradientStop>>) -> Self {
        Self::Linear(LinearGradient {
            start,
            end,
            stops: stops.into(),
        })
    }

    pub fn radial(center: Point, radius: f64, stops: impl Into<Vec<GradientStop>>) -> Self {
        Self::Radial(RadialGradient {
            center,
            radius,
            stops: stops.into(),
        })
    }
}

impl From<Color> for Paint {
    fn from(color: Color) -> Self {
        Self::Solid(color)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// The outline drawn by a stroke.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    /// Alternating dash and gap lengths; empty draws a solid line.
    pub dashes: Vec<f64>,
    pub dash_offset: f64,
}

impl StrokeStyle {
    pub fn new(width: f64) -> Self {
        Self {
            width,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dashes: Vec::new(),
            dash_offset: 0.0,
        }
    }

    pub fn cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    pub fn join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    pub fn miter_limit(mut self, limit: f64) -> Self {
        self.miter_limit = limit;
        self
    }

    pub fn dashes(mut self, dashes: impl Into<Vec<f64>>, offset: f64) -> Self {
        self.dashes = dashes.into();
        self.dash_offset = offset;
        self
    }
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self::new(1.0)
    }
}
