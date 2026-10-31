//! Rich text as plain text with styled runs and per-paragraph styles.

/// The role of a paragraph, which sets its base font.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParagraphStyle {
    Title,
    Heading,
    Subheading,
    #[default]
    Body,
    Monospaced,
}

impl ParagraphStyle {
    pub const ALL: [Self; 5] = [
        Self::Title,
        Self::Heading,
        Self::Subheading,
        Self::Body,
        Self::Monospaced,
    ];

    /// Returns whether the base font is already bold, so a bold span adds nothing.
    pub const fn is_bold(self) -> bool {
        matches!(self, Self::Title | Self::Heading | Self::Subheading)
    }
}

/// Character formatting inside a paragraph.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub monospace: bool,
    pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Run {
    /// Length in bytes of `AttributedText::text`.
    len: usize,
    style: SpanStyle,
}

/// Text with character runs and one [`ParagraphStyle`] per paragraph.
///
/// Paragraphs are separated by `'\n'`, which belongs to the paragraph it ends.
/// Adjacent runs with equal styles are merged, so equal texts compare equal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributedText {
    text: String,
    runs: Vec<Run>,
    paragraphs: Vec<ParagraphStyle>,
}

impl Default for AttributedText {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for AttributedText {
    fn from(text: &str) -> Self {
        let mut attributed = Self::new();
        attributed.push(text, SpanStyle::default());
        attributed
    }
}

impl AttributedText {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            runs: Vec::new(),
            paragraphs: vec![ParagraphStyle::Body],
        }
    }

    /// Builds text from segments that each lie in one paragraph.
    ///
    /// A paragraph takes the style of its first segment; an empty last
    /// paragraph is [`ParagraphStyle::Body`].
    pub fn from_segments<S: AsRef<str>>(
        segments: impl IntoIterator<Item = (S, SpanStyle, ParagraphStyle)>,
    ) -> Self {
        let mut attributed = Self::new();
        let mut styled = 0;
        for (text, style, paragraph) in segments {
            let text = text.as_ref();
            if text.is_empty() {
                continue;
            }
            let current = attributed.paragraphs.len() - 1;
            if styled == current {
                attributed.paragraphs[current] = paragraph;
                styled += 1;
            }
            attributed.push(text, style);
        }
        attributed
    }

    /// Returns the plain text.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn paragraphs(&self) -> &[ParagraphStyle] {
        &self.paragraphs
    }

    /// Returns the style runs in order; they cover the whole text.
    pub fn runs(&self) -> impl Iterator<Item = (&str, &SpanStyle)> {
        let mut start = 0;
        self.runs.iter().map(move |run| {
            let text = &self.text[start..start + run.len];
            start += run.len;
            (text, &run.style)
        })
    }

    /// Returns runs split at paragraph ends, each with its paragraph's style.
    pub fn segments(&self) -> Vec<(&str, &SpanStyle, ParagraphStyle)> {
        let mut segments = Vec::new();
        let mut paragraph = 0;
        for (text, style) in self.runs() {
            for piece in text.split_inclusive('\n') {
                segments.push((piece, style, self.paragraphs[paragraph]));
                if piece.ends_with('\n') {
                    paragraph += 1;
                }
            }
        }
        segments
    }

    /// Appends `text` with `style`; each newline starts a body paragraph.
    pub fn push(&mut self, text: &str, style: SpanStyle) {
        if text.is_empty() {
            return;
        }
        self.text.push_str(text);
        match self.runs.last_mut() {
            Some(last) if last.style == style => last.len += text.len(),
            _ => self.runs.push(Run {
                len: text.len(),
                style,
            }),
        }
        let breaks = text.matches('\n').count();
        self.paragraphs
            .extend(std::iter::repeat_n(ParagraphStyle::Body, breaks));
    }

    /// Sets the style of the paragraph at `index`; out-of-range indexes are ignored.
    pub fn set_paragraph(&mut self, index: usize, style: ParagraphStyle) {
        if let Some(paragraph) = self.paragraphs.get_mut(index) {
            *paragraph = style;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold() -> SpanStyle {
        SpanStyle {
            bold: true,
            ..SpanStyle::default()
        }
    }

    // Plain text has one body paragraph per line and a single default run.
    #[test]
    fn plain_text_has_body_paragraphs() {
        let text = AttributedText::from("Title\nBody\n");
        assert_eq!(text.text(), "Title\nBody\n");
        assert_eq!(text.paragraphs(), [ParagraphStyle::Body; 3]);
        assert_eq!(text.runs().count(), 1);
    }

    // Pushing equal styles merges runs, so equal content compares equal.
    #[test]
    fn equal_adjacent_runs_merge() {
        let mut split = AttributedText::new();
        split.push("Hel", bold());
        split.push("lo", bold());
        let mut whole = AttributedText::new();
        whole.push("Hello", bold());
        assert_eq!(split, whole);
    }

    // Segments break at paragraph ends and carry each paragraph's style.
    #[test]
    fn segments_split_runs_at_paragraphs() {
        let mut text = AttributedText::new();
        text.push("Title\nbo", SpanStyle::default());
        text.push("ld", bold());
        text.set_paragraph(0, ParagraphStyle::Title);
        let segments = text
            .segments()
            .into_iter()
            .map(|(text, style, paragraph)| (text, style.bold, paragraph))
            .collect::<Vec<_>>();
        assert_eq!(
            segments,
            [
                ("Title\n", false, ParagraphStyle::Title),
                ("bo", false, ParagraphStyle::Body),
                ("ld", true, ParagraphStyle::Body),
            ]
        );
    }

    // Rebuilding from segments restores runs and paragraph styles.
    #[test]
    fn segments_round_trip() {
        let mut text = AttributedText::new();
        text.push("Title\n", SpanStyle::default());
        text.push("code", bold());
        text.push("\n", SpanStyle::default());
        text.set_paragraph(0, ParagraphStyle::Title);
        text.set_paragraph(1, ParagraphStyle::Monospaced);
        let rebuilt = AttributedText::from_segments(
            text.segments()
                .into_iter()
                .map(|(text, style, paragraph)| (text, style.clone(), paragraph)),
        );
        assert_eq!(rebuilt, text);
        assert_eq!(rebuilt.paragraphs()[2], ParagraphStyle::Body);
    }
}
