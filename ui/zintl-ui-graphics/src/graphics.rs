use crate::{
    Color, FillRule, Font, HorizontalAlign, Image, Paint, Path, Point, Rect, StrokeStyle,
    TextAlign, TextMetrics, Transform, VerticalAlign,
};

/// The drawing operations a native graphics API provides.
///
/// Every call happens between the backend's begin and end of one frame.
/// State changes are scoped by `save` / `restore` like Core Graphics and
/// HTML Canvas.
pub trait GraphicsBackend {
    fn save(&mut self);
    fn restore(&mut self);
    /// Applies `transform` before the current transform.
    fn concat(&mut self, transform: Transform);
    fn set_alpha(&mut self, alpha: f64);
    fn set_antialias(&mut self, enabled: bool);
    /// Intersects the clip with `path`.
    fn clip(&mut self, path: &Path, rule: FillRule);
    /// Replaces the pixels in `rect`, including their alpha, with `color`.
    fn clear(&mut self, rect: Rect, color: Color);
    fn fill(&mut self, path: &Path, paint: &Paint, rule: FillRule);
    fn stroke(&mut self, path: &Path, paint: &Paint, style: &StrokeStyle);
    /// Draws one line of text whose baseline starts at `origin`.
    fn fill_text(&mut self, text: &str, origin: Point, font: &Font, color: Color);
    fn measure_text(&mut self, text: &str, font: &Font) -> TextMetrics;
    fn draw_image(&mut self, image: &Image, rect: Rect);
}

/// An immediate-mode 2D drawing context for one frame of a canvas.
pub struct Graphics2D<'a> {
    backend: &'a mut dyn GraphicsBackend,
    bounds: Rect,
    scale: f64,
}

impl<'a> Graphics2D<'a> {
    /// Wraps `backend`, whose canvas covers `bounds` at `scale` pixels per point.
    pub fn new(backend: &'a mut dyn GraphicsBackend, bounds: Rect, scale: f64) -> Self {
        Self {
            backend,
            bounds,
            scale,
        }
    }

    /// Returns the canvas bounds in logical points; the origin is the top-left.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Returns the number of device pixels per logical point.
    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// Runs `draw` with a copy of the current state that is restored afterwards.
    pub fn save<R>(&mut self, draw: impl FnOnce(&mut Self) -> R) -> R {
        self.backend.save();
        let result = draw(self);
        self.backend.restore();
        result
    }

    pub fn translate(&mut self, x: f64, y: f64) {
        self.backend.concat(Transform::translation(x, y));
    }

    pub fn scale_by(&mut self, x: f64, y: f64) {
        self.backend.concat(Transform::scale(x, y));
    }

    pub fn rotate(&mut self, radians: f64) {
        self.backend.concat(Transform::rotation(radians));
    }

    pub fn transform(&mut self, transform: Transform) {
        self.backend.concat(transform);
    }

    /// Multiplies the alpha of later drawing until the state is restored.
    pub fn set_alpha(&mut self, alpha: f64) {
        self.backend.set_alpha(alpha.clamp(0.0, 1.0));
    }

    pub fn set_antialias(&mut self, enabled: bool) {
        self.backend.set_antialias(enabled);
    }

    pub fn clip_rect(&mut self, rect: Rect) {
        self.backend.clip(&Path::rect(rect), FillRule::NonZero);
    }

    pub fn clip_path(&mut self, path: &Path, rule: FillRule) {
        self.backend.clip(path, rule);
    }

    /// Replaces the canvas bounds with `color`; transparent colors erase them.
    ///
    /// The bounds are interpreted in the current coordinate space, so call this
    /// before applying transforms.
    pub fn clear(&mut self, color: Color) {
        self.backend.clear(self.bounds, color);
    }

    pub fn clear_rect(&mut self, rect: Rect, color: Color) {
        self.backend.clear(rect, color);
    }

    pub fn fill_rect(&mut self, rect: Rect, paint: impl Into<Paint>) {
        self.backend
            .fill(&Path::rect(rect), &paint.into(), FillRule::NonZero);
    }

    pub fn stroke_rect(&mut self, rect: Rect, paint: impl Into<Paint>, style: &StrokeStyle) {
        self.backend.stroke(&Path::rect(rect), &paint.into(), style);
    }

    pub fn fill_path(&mut self, path: &Path, paint: impl Into<Paint>) {
        self.backend.fill(path, &paint.into(), FillRule::NonZero);
    }

    pub fn fill_path_with_rule(&mut self, path: &Path, paint: impl Into<Paint>, rule: FillRule) {
        self.backend.fill(path, &paint.into(), rule);
    }

    pub fn stroke_path(&mut self, path: &Path, paint: impl Into<Paint>, style: &StrokeStyle) {
        self.backend.stroke(path, &paint.into(), style);
    }

    pub fn stroke_line(
        &mut self,
        from: Point,
        to: Point,
        paint: impl Into<Paint>,
        style: &StrokeStyle,
    ) {
        self.backend
            .stroke(&Path::line(from, to), &paint.into(), style);
    }

    pub fn fill_ellipse(&mut self, rect: Rect, paint: impl Into<Paint>) {
        self.backend
            .fill(&Path::ellipse(rect), &paint.into(), FillRule::NonZero);
    }

    /// Draws one line of text whose baseline starts at `origin`.
    pub fn fill_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        self.backend.fill_text(text, origin, font, color);
    }

    pub fn measure_text(&mut self, text: &str, font: &Font) -> TextMetrics {
        self.backend.measure_text(text, font)
    }

    /// Draws one line of text with its `align` point at `point`; returns the line's bounds.
    pub fn fill_text_aligned(
        &mut self,
        text: &str,
        point: Point,
        align: TextAlign,
        font: &Font,
        color: Color,
    ) -> Rect {
        let metrics = self.measure_text(text, font);
        let x = match align.horizontal {
            HorizontalAlign::Left => point.x,
            HorizontalAlign::Center => point.x - metrics.width / 2.0,
            HorizontalAlign::Right => point.x - metrics.width,
        };
        let baseline = match align.vertical {
            VerticalAlign::Top => point.y + metrics.ascent,
            VerticalAlign::Center => point.y + (metrics.ascent - metrics.descent) / 2.0,
            VerticalAlign::Baseline => point.y,
            VerticalAlign::Bottom => point.y - metrics.descent,
        };
        self.fill_text(text, Point::new(x, baseline), font, color);
        Rect::new(
            x,
            baseline - metrics.ascent,
            metrics.width,
            metrics.height(),
        )
    }

    /// Draws `image` scaled into `rect`.
    pub fn draw_image(&mut self, image: &Image, rect: Rect) {
        self.backend.draw_image(image, rect);
    }
}
