//! Text sizes estimated without asking the backend.

use zintl_ui_backend_core::graphics::Font;
use zintl_ui_layout::Size;

/// The approximate size of text, computed without the backend.
///
/// Views use it for minimum sizes and fixed columns before the backend has
/// measured anything; AppKit raises control minimums to the measured size.
/// It is an estimate: East Asian wide characters take a full em, and every
/// other character an average Latin glyph's width.
pub struct TextEstimate;

impl TextEstimate {
    /// The size of the system font controls and labels use by default.
    pub const SYSTEM_FONT_SIZE: f64 = 13.0;
    /// An average Latin glyph's width as a fraction of the font size.
    const LATIN_WIDTH: f32 = 0.54;
    /// A line's height as a multiple of the font size.
    const LINE_HEIGHT: f32 = 1.25;

    /// Returns the system font at its default size.
    pub fn system_font() -> Font {
        Font::system(Self::SYSTEM_FONT_SIZE)
    }

    /// Estimates the width of `text` on one line.
    pub fn width(text: &str, font: &Font) -> f32 {
        let size = font.size() as f32;
        text.chars()
            .map(|character| {
                if Self::is_wide(character) {
                    size
                } else {
                    size * Self::LATIN_WIDTH
                }
            })
            .sum()
    }

    /// Estimates the height of one line of `font`.
    pub fn line_height(font: &Font) -> f32 {
        font.size() as f32 * Self::LINE_HEIGHT
    }

    /// Estimates the size of `text`, wrapped to `wrap_width` when given.
    ///
    /// Line breaks in `text` start new lines either way.
    pub fn size(text: &str, font: &Font, wrap_width: Option<f32>) -> Size {
        let mut width: f32 = 0.0;
        let mut lines = 0_usize;
        for line in text.split('\n') {
            let line_width = Self::width(line, font);
            match wrap_width {
                Some(wrap) if wrap > 0.0 && line_width > wrap => {
                    lines += (line_width / wrap).ceil() as usize;
                    width = width.max(wrap);
                }
                _ => {
                    lines += 1;
                    width = width.max(line_width);
                }
            }
        }
        Size::new(width, lines as f32 * Self::line_height(font))
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
    fn wide_characters_take_a_full_em() {
        // Verifies kanji take the font size each while Latin letters take about half.
        let font = TextEstimate::system_font();

        assert_eq!(TextEstimate::width("給与", &font), 26.0);
        assert!(TextEstimate::width("Food", &font) < TextEstimate::width("給与給与", &font));
        assert_eq!(TextEstimate::width("", &font), 0.0);
    }

    #[test]
    fn size_counts_line_breaks_and_wrapped_lines() {
        // Verifies explicit line breaks and wrapping both add lines, and wrapping caps the width.
        let font = Font::system(10.0);
        let line = TextEstimate::line_height(&font);

        assert_eq!(
            TextEstimate::size("ab\nc", &font, None),
            Size::new(10.8, 2.0 * line)
        );
        // Ten wide characters are 100 points, so a 40-point width takes three lines.
        let wrapped = TextEstimate::size(&"あ".repeat(10), &font, Some(40.0));
        assert_eq!(wrapped, Size::new(40.0, 3.0 * line));
    }
}
