use std::f64::consts::{FRAC_PI_2, TAU};

use crate::{Point, Rect};

/// One drawing instruction of a [`Path`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathElement {
    MoveTo(Point),
    LineTo(Point),
    /// A quadratic Bézier curve through a control point.
    QuadTo(Point, Point),
    /// A cubic Bézier curve through two control points.
    CubicTo(Point, Point, Point),
    Close,
}

/// How a fill decides which regions of a self-intersecting path are inside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// A platform-neutral vector path built from lines and Bézier curves.
///
/// Arcs and ellipses are converted to cubic curves, so every backend only
/// needs the elements in [`PathElement`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    elements: Vec<PathElement>,
    /// Whether the last element leaves an open subpath to continue.
    open: bool,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rect(rect: Rect) -> Self {
        let mut path = Self::new();
        path.add_rect(rect);
        path
    }

    pub fn rounded_rect(rect: Rect, radius: f64) -> Self {
        let mut path = Self::new();
        path.add_rounded_rect(rect, radius);
        path
    }

    pub fn ellipse(rect: Rect) -> Self {
        let mut path = Self::new();
        path.add_ellipse(rect);
        path
    }

    pub fn circle(center: Point, radius: f64) -> Self {
        Self::ellipse(Rect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        ))
    }

    /// Returns a single line segment.
    pub fn line(from: Point, to: Point) -> Self {
        let mut path = Self::new();
        path.move_to(from).line_to(to);
        path
    }

    pub fn elements(&self) -> &[PathElement] {
        &self.elements
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn move_to(&mut self, point: Point) -> &mut Self {
        self.elements.push(PathElement::MoveTo(point));
        self.open = true;
        self
    }

    /// Adds a line; an empty path starts its first subpath at `point`.
    pub fn line_to(&mut self, point: Point) -> &mut Self {
        if !self.open {
            return self.move_to(point);
        }
        self.elements.push(PathElement::LineTo(point));
        self
    }

    pub fn quad_to(&mut self, control: Point, point: Point) -> &mut Self {
        if !self.open {
            self.move_to(control);
        }
        self.elements.push(PathElement::QuadTo(control, point));
        self
    }

    pub fn cubic_to(&mut self, first: Point, second: Point, point: Point) -> &mut Self {
        if !self.open {
            self.move_to(first);
        }
        self.elements
            .push(PathElement::CubicTo(first, second, point));
        self
    }

    pub fn close(&mut self) -> &mut Self {
        if self.open {
            self.elements.push(PathElement::Close);
            self.open = false;
        }
        self
    }

    /// Adds a circular arc that starts at `start` radians and sweeps by `sweep`.
    ///
    /// Positive angles turn from +x toward +y, which is clockwise on screen.
    /// Like HTML Canvas, a line connects the current point to the arc start.
    pub fn arc(&mut self, center: Point, radius: f64, start: f64, sweep: f64) -> &mut Self {
        let sweep = sweep.clamp(-TAU, TAU);
        let point_at = |angle: f64| {
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        };
        self.line_to(point_at(start));
        // Each cubic segment approximates at most a quarter turn.
        let segments = (sweep.abs() / FRAC_PI_2).ceil().max(1.0) as usize;
        let step = sweep / segments as f64;
        let handle = 4.0 / 3.0 * (step / 4.0).tan() * radius;
        let mut angle = start;
        for _ in 0..segments {
            let next = angle + step;
            let (sin, cos) = angle.sin_cos();
            let (next_sin, next_cos) = next.sin_cos();
            let from = point_at(angle);
            let to = point_at(next);
            self.elements.push(PathElement::CubicTo(
                Point::new(from.x - handle * sin, from.y + handle * cos),
                Point::new(to.x + handle * next_sin, to.y - handle * next_cos),
                to,
            ));
            angle = next;
        }
        self
    }

    pub fn add_rect(&mut self, rect: Rect) -> &mut Self {
        self.move_to(rect.origin())
            .line_to(Point::new(rect.max_x(), rect.y))
            .line_to(Point::new(rect.max_x(), rect.max_y()))
            .line_to(Point::new(rect.x, rect.max_y()))
            .close()
    }

    pub fn add_rounded_rect(&mut self, rect: Rect, radius: f64) -> &mut Self {
        let radius = radius.min(rect.width / 2.0).min(rect.height / 2.0).max(0.0);
        if radius == 0.0 {
            return self.add_rect(rect);
        }
        self.close();
        self.move_to(Point::new(rect.x + radius, rect.y));
        self.arc(
            Point::new(rect.max_x() - radius, rect.y + radius),
            radius,
            -FRAC_PI_2,
            FRAC_PI_2,
        );
        self.arc(
            Point::new(rect.max_x() - radius, rect.max_y() - radius),
            radius,
            0.0,
            FRAC_PI_2,
        );
        self.arc(
            Point::new(rect.x + radius, rect.max_y() - radius),
            radius,
            FRAC_PI_2,
            FRAC_PI_2,
        );
        self.arc(
            Point::new(rect.x + radius, rect.y + radius),
            radius,
            std::f64::consts::PI,
            FRAC_PI_2,
        );
        self.close()
    }

    pub fn add_ellipse(&mut self, rect: Rect) -> &mut Self {
        // Four quarter arcs with the standard 0.5523 handle ratio.
        const KAPPA: f64 = 0.552_284_749_830_793_4;
        let center = rect.center();
        let (rx, ry) = (rect.width / 2.0, rect.height / 2.0);
        let (ox, oy) = (rx * KAPPA, ry * KAPPA);
        self.close();
        self.move_to(Point::new(rect.max_x(), center.y));
        self.cubic_to(
            Point::new(rect.max_x(), center.y + oy),
            Point::new(center.x + ox, rect.max_y()),
            Point::new(center.x, rect.max_y()),
        );
        self.cubic_to(
            Point::new(center.x - ox, rect.max_y()),
            Point::new(rect.x, center.y + oy),
            Point::new(rect.x, center.y),
        );
        self.cubic_to(
            Point::new(rect.x, center.y - oy),
            Point::new(center.x - ox, rect.y),
            Point::new(center.x, rect.y),
        );
        self.cubic_to(
            Point::new(center.x + ox, rect.y),
            Point::new(rect.max_x(), center.y - oy),
            Point::new(rect.max_x(), center.y),
        );
        self.close()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last_point(path: &Path) -> Point {
        match path
            .elements()
            .iter()
            .rev()
            .find(|element| **element != PathElement::Close)
        {
            Some(PathElement::MoveTo(point) | PathElement::LineTo(point)) => *point,
            Some(PathElement::QuadTo(_, point) | PathElement::CubicTo(_, _, point)) => *point,
            _ => panic!("the path has no points"),
        }
    }

    #[test]
    fn first_line_starts_a_subpath() {
        // Verifies drawing into an empty path begins with a move, not a dangling line.
        let mut path = Path::new();
        path.line_to(Point::new(1.0, 2.0));
        assert_eq!(
            path.elements(),
            &[PathElement::MoveTo(Point::new(1.0, 2.0))]
        );
    }

    #[test]
    fn arc_ends_on_the_circle() {
        // Verifies a half-turn arc ends at the opposite point of the circle.
        let mut path = Path::new();
        path.arc(Point::new(10.0, 10.0), 5.0, 0.0, std::f64::consts::PI);
        let end = last_point(&path);
        assert!((end.x - 5.0).abs() < 1e-9 && (end.y - 10.0).abs() < 1e-9);
        // Two quarter-turn segments follow the initial move.
        assert_eq!(path.elements().len(), 3);
    }

    #[test]
    fn rounded_rect_without_radius_is_a_rect() {
        // Verifies a zero radius produces the same elements as a plain rectangle.
        let rect = Rect::new(0.0, 0.0, 4.0, 3.0);
        assert_eq!(Path::rounded_rect(rect, 0.0), Path::rect(rect));
    }

    #[test]
    fn shapes_close_their_subpaths() {
        // Verifies built-in shapes end closed so later lines start new subpaths.
        let mut path = Path::ellipse(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(path.elements().last(), Some(&PathElement::Close));
        path.line_to(Point::new(1.0, 1.0));
        assert_eq!(
            path.elements().last(),
            Some(&PathElement::MoveTo(Point::new(1.0, 1.0)))
        );
    }
}
