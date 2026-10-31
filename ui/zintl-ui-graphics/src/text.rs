use std::sync::Arc;

/// A font weight from 1 to 1000, where 400 is regular and 700 is bold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const LIGHT: Self = Self(300);
    pub const REGULAR: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMIBOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const HEAVY: Self = Self(800);
    pub const BLACK: Self = Self(900);
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::REGULAR
    }
}

/// A font description that backends resolve to a native font.
///
/// A missing or unknown family falls back to the system UI font.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Font {
    pub family: Option<Arc<str>>,
    /// Size in logical points, stored as bits so the description can be hashed.
    size_bits: u64,
    pub weight: FontWeight,
    pub italic: bool,
}

impl Font {
    pub fn system(size: f64) -> Self {
        Self {
            family: None,
            size_bits: size.to_bits(),
            weight: FontWeight::REGULAR,
            italic: false,
        }
    }

    pub fn named(family: impl Into<Arc<str>>, size: f64) -> Self {
        Self {
            family: Some(family.into()),
            ..Self::system(size)
        }
    }

    pub fn size(&self) -> f64 {
        f64::from_bits(self.size_bits)
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn bold(self) -> Self {
        self.weight(FontWeight::BOLD)
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

/// The extent of a line of text drawn at a baseline origin.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    /// The advance width.
    pub width: f64,
    /// Distance from the baseline up to the top of the line.
    pub ascent: f64,
    /// Distance from the baseline down to the bottom of the line.
    pub descent: f64,
    pub leading: f64,
}

impl TextMetrics {
    pub fn height(&self) -> f64 {
        self.ascent + self.descent
    }
}
