//! Conversion between styled runs and `NSAttributedString`.

use crate::graphics::Font;
use crate::native;
use zpd_objc::{Id, Strong};

/// The role of a paragraph, which sets its base font.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RichParagraph {
    Title,
    Heading,
    Subheading,
    #[default]
    Body,
    Monospaced,
}

impl RichParagraph {
    const ALL: [Self; 5] = [
        Self::Title,
        Self::Heading,
        Self::Subheading,
        Self::Body,
        Self::Monospaced,
    ];

    /// Returns the value stored in the paragraph attribute and menu item tags.
    pub fn tag(self) -> native::Integer {
        Self::ALL
            .iter()
            .position(|style| *style == self)
            .unwrap_or(3) as native::Integer
    }

    pub fn from_tag(tag: native::Integer) -> Option<Self> {
        usize::try_from(tag)
            .ok()
            .and_then(|tag| Self::ALL.get(tag).copied())
    }

    /// Returns the base point size and CSS weight.
    fn base(self) -> (f64, u16) {
        match self {
            Self::Title => (22.0, 700),
            Self::Heading => (17.0, 700),
            Self::Subheading => (14.0, 600),
            Self::Body => (14.0, 400),
            Self::Monospaced => (13.0, 400),
        }
    }

    /// Returns whether bold is part of the base font rather than a span style.
    fn is_bold(self) -> bool {
        self.base().1 >= 600
    }
}

/// The formatting of one run of text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RichStyle {
    pub paragraph: RichParagraph,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub monospace: bool,
    pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichRun {
    pub text: String,
    pub style: RichStyle,
}

/// `NSAttributedString` keys and conversions for [`RichRun`]s.
pub(crate) struct RichText;

impl RichText {
    /// Marks the paragraph style, since a font alone cannot tell a bold body from a heading.
    const PARAGRAPH_KEY: &str = "ZintlParagraphStyle";
    const FONT_ITALIC_TRAIT: u32 = 1 << 0;
    const FONT_BOLD_TRAIT: u32 = 1 << 1;
    const FONT_MONOSPACE_TRAIT: u32 = 1 << 10;
    /// NSFontTraitMask values for `convertFont:toHaveTrait:`.
    const ITALIC_MASK: u64 = 1;
    const BOLD_MASK: u64 = 2;

    /// Builds an attributed string from `runs`.
    pub(crate) fn attributed_string(runs: &[RichRun]) -> Strong {
        let result = native::alloc_init(zpd_objc::class!("NSMutableAttributedString"));
        for run in runs {
            let text = native::nsstring(&run.text);
            let attributes = Self::attributes(&run.style);
            // SAFETY: The string and dictionary are live; the result copies them.
            unsafe {
                let allocated = zpd_objc::msg_send!(zpd_objc::class!("NSAttributedString"), zpd_objc::sel!("alloc"), () => Id);
                let piece = Strong::from_retained(zpd_objc::msg_send!(allocated, zpd_objc::sel!("initWithString:attributes:"), ((text.as_ptr()): Id, (attributes.as_ptr()): Id) => Id))
                    .expect("NSAttributedString allocation failed");
                zpd_objc::msg_send!(result.as_ptr(), zpd_objc::sel!("appendAttributedString:"), ((piece.as_ptr()): Id) => ());
            }
        }
        result
    }

    /// Returns the attribute dictionary for `style`.
    pub(crate) fn attributes(style: &RichStyle) -> Strong {
        let attributes = native::alloc_init(zpd_objc::class!("NSMutableDictionary"));
        let set = |key: &str, value: Id| {
            let key = native::nsstring(key);
            // SAFETY: The dictionary retains the live value and copies the key.
            unsafe {
                zpd_objc::msg_send!(attributes.as_ptr(), zpd_objc::sel!("setObject:forKey:"), ((value): Id, (key.as_ptr()): Id) => ());
            }
        };
        let font = Self::font(style);
        set("NSFont", font.as_ptr());
        // SAFETY: These class methods return autoreleased objects that `set` retains.
        unsafe {
            set(
                "NSColor",
                zpd_objc::msg_send!(zpd_objc::class!("NSColor"), zpd_objc::sel!("textColor"), () => Id),
            );
            let number = |value: native::Integer| zpd_objc::msg_send!(zpd_objc::class!("NSNumber"), zpd_objc::sel!("numberWithInteger:"), ((value): native::Integer) => Id);
            set(Self::PARAGRAPH_KEY, number(style.paragraph.tag()));
            if style.underline {
                set("NSUnderline", number(1));
            }
            if style.strikethrough {
                set("NSStrikethrough", number(1));
            }
        }
        if let Some(link) = &style.link {
            set("NSLink", native::nsstring(link).as_ptr());
        }
        attributes
    }

    /// Returns the font for a run: the paragraph's base font with the run's traits.
    fn font(style: &RichStyle) -> Strong {
        let (size, base_weight) = style.paragraph.base();
        let bold = style.bold && !style.paragraph.is_bold();
        let monospace = style.monospace || style.paragraph == RichParagraph::Monospaced;
        if !monospace {
            let weight = if bold { 700 } else { base_weight };
            let font = Font::new(None, size, weight, style.italic);
            // SAFETY: `font` keeps the NSFont alive while it is retained here.
            return unsafe { Strong::retain(font.as_ptr()) }.expect("fonts are non-nil");
        }
        // SAFETY: The monospaced system font exists; the font manager converts it on the main thread.
        unsafe {
            let mut font = zpd_objc::msg_send!(zpd_objc::class!("NSFont"), zpd_objc::sel!("monospacedSystemFontOfSize:weight:"), ((size): f64, (native::NSFontWeightRegular): f64) => Id);
            let manager = zpd_objc::msg_send!(zpd_objc::class!("NSFontManager"), zpd_objc::sel!("sharedFontManager"), () => Id);
            for (enabled, mask) in [(bold, Self::BOLD_MASK), (style.italic, Self::ITALIC_MASK)] {
                if enabled {
                    font = zpd_objc::msg_send!(manager, zpd_objc::sel!("convertFont:toHaveTrait:"), ((font): Id, (mask): u64) => Id);
                }
            }
            Strong::retain(font).expect("fonts are non-nil")
        }
    }

    /// Reads the runs of an attributed string, merging equal neighbors.
    ///
    /// # Safety
    /// `attributed` must be a live `NSAttributedString`.
    pub(crate) unsafe fn runs(attributed: Id) -> Vec<RichRun> {
        let mut runs: Vec<RichRun> = Vec::new();
        // SAFETY: Guaranteed by the caller; ranges come from the string itself.
        unsafe {
            let string = zpd_objc::msg_send!(attributed, zpd_objc::sel!("string"), () => Id);
            let length = zpd_objc::msg_send!(attributed, zpd_objc::sel!("length"), () => u64);
            let mut index = 0;
            while index < length {
                let mut range = native::Range {
                    location: 0,
                    length: 0,
                };
                let attributes = zpd_objc::msg_send!(attributed, zpd_objc::sel!("attributesAtIndex:effectiveRange:"), ((index): u64, (&mut range): *mut native::Range) => Id);
                let text = native::rust_string(
                    zpd_objc::msg_send!(string, zpd_objc::sel!("substringWithRange:"), ((range): native::Range) => Id),
                );
                let style = Self::style(attributes);
                match runs.last_mut() {
                    Some(last) if last.style == style => last.text.push_str(&text),
                    _ => runs.push(RichRun { text, style }),
                }
                index = range.location + range.length.max(1);
            }
        }
        runs
    }

    /// Reads the style from an attribute dictionary.
    ///
    /// # Safety
    /// `attributes` must be a live `NSDictionary`.
    pub(crate) unsafe fn style(attributes: Id) -> RichStyle {
        // SAFETY: Guaranteed by the caller; values are checked for nil and class.
        unsafe {
            let get = |key: &str| {
                let key = native::nsstring(key);
                zpd_objc::msg_send!(attributes, zpd_objc::sel!("objectForKey:"), ((key.as_ptr()): Id) => Id)
            };
            let integer = |value: Id| {
                (!value.is_null())
                    .then(|| zpd_objc::msg_send!(value, zpd_objc::sel!("integerValue"), () => native::Integer))
                    .unwrap_or(0)
            };
            let paragraph = get(Self::PARAGRAPH_KEY);
            let paragraph = if paragraph.is_null() {
                RichParagraph::Body
            } else {
                RichParagraph::from_tag(integer(paragraph)).unwrap_or_default()
            };
            let font = get("NSFont");
            let traits = if font.is_null() {
                0
            } else {
                let descriptor =
                    zpd_objc::msg_send!(font, zpd_objc::sel!("fontDescriptor"), () => Id);
                zpd_objc::msg_send!(descriptor, zpd_objc::sel!("symbolicTraits"), () => u32)
            };
            let link = get("NSLink");
            let link = if link.is_null() {
                None
            } else if zpd_objc::msg_send!(link, zpd_objc::sel!("isKindOfClass:"), ((zpd_objc::class!("NSURL")): Id) => bool)
            {
                Some(native::rust_string(
                    zpd_objc::msg_send!(link, zpd_objc::sel!("absoluteString"), () => Id),
                ))
            } else {
                Some(native::rust_string(
                    zpd_objc::msg_send!(link, zpd_objc::sel!("description"), () => Id),
                ))
            };
            RichStyle {
                paragraph,
                bold: traits & Self::FONT_BOLD_TRAIT != 0 && !paragraph.is_bold(),
                italic: traits & Self::FONT_ITALIC_TRAIT != 0,
                underline: integer(get("NSUnderline")) != 0,
                strikethrough: integer(get("NSStrikethrough")) != 0,
                monospace: traits & Self::FONT_MONOSPACE_TRAIT != 0
                    && paragraph != RichParagraph::Monospaced,
                link,
            }
        }
    }
}
