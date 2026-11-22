//! Text sizes estimated without asking the backend.

/// The approximate size of text, computed without the backend.
///
/// Views use it for minimum sizes and fixed columns before the backend has
/// measured anything; AppKit raises control minimums to the measured size.
/// It is an estimate: East Asian wide characters take a full em, and every
/// other character an average Latin glyph's width.
pub struct TextEstimate;

impl TextEstimate {
    /// The size of the system font controls and labels use by default, in points.
    pub const SYSTEM_FONT_SIZE: f32 = 13.0;
    /// An average Latin glyph's width as a fraction of the font size.
    const LATIN_WIDTH: f32 = 0.54;
    /// A line's height as a multiple of the font size.
    const LINE_HEIGHT: f32 = 1.25;

    /// Estimates the width of `text` on one line at `font_size` points.
    ///
    /// The weight and family do not change the estimate.
    pub fn width(text: &str, font_size: f32) -> f32 {
        text.chars()
            .map(|character| {
                if Self::is_wide(character) {
                    font_size
                } else {
                    font_size * Self::LATIN_WIDTH
                }
            })
            .sum()
    }

    /// Estimates the height of one line at `font_size` points.
    pub fn line_height(font_size: f32) -> f32 {
        font_size * Self::LINE_HEIGHT
    }

    /// Returns whether `character` is East Asian wide or full-width, such as kanji or kana.
    fn is_wide(character: char) -> bool {
        matches!(
            character as u32,
            0x1100..=0x115F
                | 0x2E80..=0x303E
                | 0x3041..=0x33FF
                | 0x3400..=0x4DBF
                | 0x4E00..=0x9FFF
                | 0xA000..=0xA4CF
                | 0xAC00..=0xD7A3
                | 0xF900..=0xFAFF
                | 0xFE30..=0xFE4F
                | 0xFF00..=0xFF60
                | 0xFFE0..=0xFFE6
                | 0x1F300..=0x1F64F
                | 0x1F900..=0x1F9FF
                | 0x20000..=0x2FFFD
                | 0x30000..=0x3FFFD
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_characters_take_a_full_em_and_others_about_half() {
        // Verifies kanji take the font size each and ASCII letters 0.54 em.
        let size = TextEstimate::SYSTEM_FONT_SIZE;

        assert_eq!(TextEstimate::width("給与", size), 26.0);
        assert_eq!(TextEstimate::width("ab", 10.0), 10.8);
        assert_eq!(TextEstimate::width("", size), 0.0);
        assert_eq!(TextEstimate::line_height(10.0), 12.5);
    }
}
