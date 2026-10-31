/// A point in logical points / DIP.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Rectangle in logical points / DIP.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

    /// Returns a rectangle at the origin with the given size.
    pub const fn from_size(width: f64, height: f64) -> Self {
        Self::new(0.0, 0.0, width, height)
    }

    pub const fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn max_x(&self) -> f64 {
        self.x + self.width
    }

    pub fn max_y(&self) -> f64 {
        self.y + self.height
    }

    /// Returns whether `point` lies inside, including the top and left edges.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x && point.x < self.max_x() && point.y >= self.y && point.y < self.max_y()
    }

    /// Shrinks every edge by `amount`; negative amounts grow the rectangle.
    pub fn inset(&self, amount: f64) -> Self {
        Self::new(
            self.x + amount,
            self.y + amount,
            (self.width - amount * 2.0).max(0.0),
            (self.height - amount * 2.0).max(0.0),
        )
    }
}

/// An sRGB color with components in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn rgb(red: f64, green: f64, blue: f64) -> Self {
        Self::rgba(red, green, blue, 1.0)
    }

    pub const fn rgba(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Returns this color with its alpha replaced.
    pub const fn with_alpha(self, alpha: f64) -> Self {
        Self { alpha, ..self }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

/// A 2D affine transform that maps `(x, y)` to `(a*x + c*y + tx, b*x + d*y + ty)`.
///
/// The layout matches Core Graphics' `CGAffineTransform` and Direct2D's
/// `D2D1_MATRIX_3X2_F`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub const fn translation(x: f64, y: f64) -> Self {
        Self {
            tx: x,
            ty: y,
            ..Self::IDENTITY
        }
    }

    pub const fn scale(x: f64, y: f64) -> Self {
        Self {
            a: x,
            d: y,
            ..Self::IDENTITY
        }
    }

    /// Rotates by `radians`; positive angles turn from +x toward +y.
    pub fn rotation(radians: f64) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            tx: 0.0,
            ty: 0.0,
        }
    }

    /// Returns the transform that applies `self` first and then `next`.
    pub fn then(self, next: Self) -> Self {
        Self {
            a: self.a * next.a + self.b * next.c,
            b: self.a * next.b + self.b * next.d,
            c: self.c * next.a + self.d * next.c,
            d: self.c * next.b + self.d * next.d,
            tx: self.tx * next.a + self.ty * next.c + next.tx,
            ty: self.tx * next.b + self.ty * next.d + next.ty,
        }
    }

    pub fn apply(&self, point: Point) -> Point {
        Point::new(
            self.a * point.x + self.c * point.y + self.tx,
            self.b * point.x + self.d * point.y + self.ty,
        )
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_then_applies_left_to_right() {
        // Verifies `a.then(b)` maps a point through `a` before `b`.
        let transform = Transform::scale(2.0, 3.0).then(Transform::translation(10.0, 20.0));
        assert_eq!(
            transform.apply(Point::new(1.0, 1.0)),
            Point::new(12.0, 23.0)
        );
    }

    #[test]
    fn rotation_turns_x_toward_y() {
        // Verifies positive rotation maps +x onto +y in the y-down space.
        let point = Transform::rotation(std::f64::consts::FRAC_PI_2).apply(Point::new(1.0, 0.0));
        assert!((point.x).abs() < 1e-12 && (point.y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rect_contains_is_half_open() {
        // Verifies the top-left edges are inside and the bottom-right edges are outside.
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(rect.contains(Point::new(0.0, 0.0)));
        assert!(!rect.contains(Point::new(10.0, 5.0)));
    }
}
