//! Platform-neutral 2D drawing descriptions and the immediate-mode [`Graphics2D`] API.
//!
//! Native backends implement [`GraphicsBackend`] over an OS graphics API such
//! as Core Graphics or Direct2D. Coordinates are logical points with a
//! top-left origin and the y axis pointing down.

mod attributed;
mod geometry;
mod graphics;
mod image;
mod paint;
mod path;
mod recording;
mod text;

pub use attributed::{AttributedText, ParagraphStyle, SpanStyle};
pub use geometry::{Color, Point, Rect, Transform};
pub use graphics::{Graphics2D, GraphicsBackend};
pub use image::{Image, ImageError};
pub use paint::{
    GradientStop, LineCap, LineJoin, LinearGradient, Paint, RadialGradient, StrokeStyle,
};
pub use path::{FillRule, Path, PathElement};
pub use recording::{DrawCommand, RecordingBackend};
pub use text::{Font, FontWeight, TextMetrics};
