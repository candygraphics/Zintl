//! List markers and checklist behavior for rich [`TextEditor`]s.
//!
//! Markers are drawn in the head indent of list paragraphs and are not part
//! of the text, so the text stays the same as what the user typed.

use crate::native;
use zpd_objc::{Id, Strong};

use super::rich_text::{RichParagraph, RichText};
use super::text_editor::TextEditor;

/// A list paragraph and where its first line lies in the text view.
struct ListLine {
    paragraph: RichParagraph,
    range: native::Range,
    /// The first line fragment in view coordinates.
    line: native::Rect,
    /// The position among consecutive numbered paragraphs, from 1.
    number: usize,
}

/// Geometry of list markers in a TextKit 1 text view.
struct ListMarkers;

impl ListMarkers {
    const CIRCLE: f64 = 15.0;

    /// Returns the list paragraphs whose first line intersects `visible`.
    ///
    /// # Safety
    /// `text` must be a live rich text view using TextKit 1.
    unsafe fn lines(text: Id, visible: Option<native::Rect>) -> Vec<ListLine> {
        let mut lines = Vec::new();
        // SAFETY: Guaranteed by the caller; ranges come from the view's own string.
        unsafe {
            let storage = zpd_objc::msg_send!(text, zpd_objc::sel!("textStorage"), () => Id);
            let string = zpd_objc::msg_send!(storage, zpd_objc::sel!("string"), () => Id);
            let length = zpd_objc::msg_send!(storage, zpd_objc::sel!("length"), () => u64);
            let layout = zpd_objc::msg_send!(text, zpd_objc::sel!("layoutManager"), () => Id);
            let origin = zpd_objc::msg_send!(text, zpd_objc::sel!("textContainerOrigin"), () => native::Point);
            let mut number = 0;
            let mut location = 0;
            while location < length {
                let range = zpd_objc::msg_send!(string, zpd_objc::sel!("paragraphRangeForRange:"), ((native::Range { location, length: 0 }): native::Range) => native::Range);
                let mut effective = native::Range::default();
                let attributes = zpd_objc::msg_send!(storage, zpd_objc::sel!("attributesAtIndex:effectiveRange:"), ((range.location): u64, (&mut effective): *mut native::Range) => Id);
                let paragraph = RichText::style(attributes).paragraph;
                number = if paragraph == RichParagraph::Numbered {
                    number + 1
                } else {
                    0
                };
                if paragraph.is_list() {
                    let glyph = zpd_objc::msg_send!(layout, zpd_objc::sel!("glyphIndexForCharacterAtIndex:"), ((range.location): u64) => u64);
                    let mut line = zpd_objc::msg_send!(layout, zpd_objc::sel!("lineFragmentRectForGlyphAtIndex:effectiveRange:"), ((glyph): u64, (std::ptr::null_mut::<native::Range>()): *mut native::Range) => native::Rect);
                    line.origin.x += origin.x;
                    line.origin.y += origin.y;
                    let shown = visible.is_none_or(|visible| {
                        line.origin.y + line.size.height >= visible.origin.y
                            && line.origin.y <= visible.origin.y + visible.size.height
                    });
                    if shown {
                        lines.push(ListLine {
                            paragraph,
                            range,
                            line,
                            number,
                        });
                    }
                }
                location = range.location + range.length.max(1);
            }
        }
        lines
    }

    /// Returns the marker area: the head indent of the line's first fragment.
    ///
    /// # Safety
    /// `text` must be a live text view.
    unsafe fn gutter(text: Id, line: native::Rect) -> native::Rect {
        // SAFETY: Guaranteed by the caller.
        let padding = unsafe {
            let container = zpd_objc::msg_send!(text, zpd_objc::sel!("textContainer"), () => Id);
            zpd_objc::msg_send!(container, zpd_objc::sel!("lineFragmentPadding"), () => f64)
        };
        native::Rect {
            origin: native::Point {
                x: line.origin.x + padding,
                y: line.origin.y,
            },
            size: native::Size {
                width: RichParagraph::LIST_INDENT,
                height: line.size.height,
            },
        }
    }

    /// Draws the marker for `line`.
    ///
    /// # Safety
    /// `text` must be a live text view that is drawing.
    unsafe fn draw(text: Id, line: &ListLine) {
        // SAFETY: Guaranteed by the caller; drawing uses the current graphics context.
        unsafe {
            let gutter = Self::gutter(text, line.line);
            let middle = gutter.origin.y + gutter.size.height / 2.0;
            let label = match line.paragraph {
                RichParagraph::Bulleted => "•".to_owned(),
                RichParagraph::Dashed => "–".to_owned(),
                RichParagraph::Numbered => format!("{}.", line.number),
                RichParagraph::Checklist { done } => {
                    Self::draw_check(gutter.origin.x, middle, done);
                    return;
                }
                _ => return,
            };
            let attributes = native::alloc_init(zpd_objc::class!("NSMutableDictionary"));
            let font = zpd_objc::msg_send!(zpd_objc::class!("NSFont"), zpd_objc::sel!("systemFontOfSize:"), ((14.0): f64) => Id);
            let color = zpd_objc::msg_send!(zpd_objc::class!("NSColor"), zpd_objc::sel!("textColor"), () => Id);
            for (key, value) in [("NSFont", font), ("NSColor", color)] {
                zpd_objc::msg_send!(attributes.as_ptr(), zpd_objc::sel!("setObject:forKey:"), ((value): Id, (native::nsstring(key).as_ptr()): Id) => ());
            }
            let label = native::nsstring(&label);
            let size = zpd_objc::msg_send!(label.as_ptr(), zpd_objc::sel!("sizeWithAttributes:"), ((attributes.as_ptr()): Id) => native::Size);
            let point = native::Point {
                x: gutter.origin.x,
                y: middle - size.height / 2.0,
            };
            zpd_objc::msg_send!(label.as_ptr(), zpd_objc::sel!("drawAtPoint:withAttributes:"), ((point): native::Point, (attributes.as_ptr()): Id) => ());
        }
    }

    /// Draws a checklist circle, filled with a checkmark when `done`.
    ///
    /// # Safety
    /// Must run while a view is drawing.
    unsafe fn draw_check(x: f64, middle: f64, done: bool) {
        let rect = native::Rect {
            origin: native::Point {
                x: x + 0.5,
                y: middle - Self::CIRCLE / 2.0,
            },
            size: native::Size {
                width: Self::CIRCLE,
                height: Self::CIRCLE,
            },
        };
        // SAFETY: Guaranteed by the caller; paths and colors are autoreleased.
        unsafe {
            let circle = zpd_objc::msg_send!(zpd_objc::class!("NSBezierPath"), zpd_objc::sel!("bezierPathWithOvalInRect:"), ((rect): native::Rect) => Id);
            let color = |name| zpd_objc::msg_send!(zpd_objc::class!("NSColor"), name, () => Id);
            if !done {
                zpd_objc::msg_send!(color(zpd_objc::sel!("tertiaryLabelColor")), zpd_objc::sel!("setStroke"), () => ());
                zpd_objc::msg_send!(circle, zpd_objc::sel!("setLineWidth:"), ((1.0): f64) => ());
                zpd_objc::msg_send!(circle, zpd_objc::sel!("stroke"), () => ());
                return;
            }
            zpd_objc::msg_send!(color(zpd_objc::sel!("controlAccentColor")), zpd_objc::sel!("setFill"), () => ());
            zpd_objc::msg_send!(circle, zpd_objc::sel!("fill"), () => ());
            // The text view is flipped, so y grows downward.
            let check = Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("NSBezierPath"), zpd_objc::sel!("bezierPath"), () => Id))
                .expect("NSBezierPath allocation failed");
            let (left, top) = (rect.origin.x, rect.origin.y);
            let point = |x: f64, y: f64| native::Point {
                x: left + x * Self::CIRCLE,
                y: top + y * Self::CIRCLE,
            };
            zpd_objc::msg_send!(check.as_ptr(), zpd_objc::sel!("moveToPoint:"), ((point(0.28, 0.52)): native::Point) => ());
            zpd_objc::msg_send!(check.as_ptr(), zpd_objc::sel!("lineToPoint:"), ((point(0.44, 0.68)): native::Point) => ());
            zpd_objc::msg_send!(check.as_ptr(), zpd_objc::sel!("lineToPoint:"), ((point(0.72, 0.34)): native::Point) => ());
            zpd_objc::msg_send!(check.as_ptr(), zpd_objc::sel!("setLineWidth:"), ((1.8): f64) => ());
            zpd_objc::msg_send!(check.as_ptr(), zpd_objc::sel!("setLineCapStyle:"), ((1_u64): u64) => ());
            zpd_objc::msg_send!(color(zpd_objc::sel!("whiteColor")), zpd_objc::sel!("setStroke"), () => ());
            zpd_objc::msg_send!(check.as_ptr(), zpd_objc::sel!("stroke"), () => ());
        }
    }
}

/// Draws list markers after the background, before the text.
pub(super) unsafe extern "C" fn draw_background(
    object: Id,
    selector: zpd_objc::Sel,
    rect: native::Rect,
) {
    // SAFETY: AppKit calls this on the live text view while it draws.
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSTextView"), selector, ((rect): native::Rect) => ());
    }
    zpd_objc::abort_on_unwind(|| {
        // SAFETY: The receiver is a live text view using TextKit 1.
        unsafe {
            for line in ListMarkers::lines(object, Some(rect)) {
                ListMarkers::draw(object, &line);
            }
        }
    });
}

/// Toggles a checklist item when its circle is clicked.
pub(super) unsafe extern "C" fn mouse_down(object: Id, selector: zpd_objc::Sel, event: Id) {
    let handled = zpd_objc::abort_on_unwind(|| {
        // SAFETY: AppKit sends the live mouse event to the live text view.
        unsafe {
            let location =
                zpd_objc::msg_send!(event, zpd_objc::sel!("locationInWindow"), () => native::Point);
            let point = zpd_objc::msg_send!(object, zpd_objc::sel!("convertPoint:fromView:"), ((location): native::Point, (zpd_objc::NIL): Id) => native::Point);
            let hit = ListMarkers::lines(object, None).into_iter().find(|line| {
                let gutter = ListMarkers::gutter(object, line.line);
                matches!(line.paragraph, RichParagraph::Checklist { .. })
                    && point.x >= gutter.origin.x
                    && point.x < gutter.origin.x + gutter.size.width
                    && point.y >= gutter.origin.y
                    && point.y < gutter.origin.y + gutter.size.height
            });
            let Some(line) = hit else {
                return false;
            };
            TextEditor::restyle(
                object,
                native::Range {
                    location: line.range.location,
                    length: 0,
                },
                |paragraph| match paragraph {
                    RichParagraph::Checklist { done } => RichParagraph::Checklist { done: !done },
                    other => other,
                },
                false,
            );
            true
        }
    });
    if !handled {
        // SAFETY: NSTextView handles clicks outside checklist circles.
        unsafe {
            zpd_objc::msg_send_super!(object, zpd_objc::class!("NSTextView"), selector, ((event): Id) => ());
        }
    }
}

/// Continues lists on Return, ends them on an empty item, and returns to body
/// text after a heading.
pub(super) unsafe extern "C" fn insert_newline(object: Id, selector: zpd_objc::Sel, sender: Id) {
    // SAFETY: AppKit sends this action to the live text view; ranges come from its string.
    let (current, empty) = zpd_objc::abort_on_unwind(|| unsafe {
        let current = TextEditor::paragraph_at_selection(object);
        let string = zpd_objc::msg_send!(object, zpd_objc::sel!("string"), () => Id);
        let selection =
            zpd_objc::msg_send!(object, zpd_objc::sel!("selectedRange"), () => native::Range);
        let range = zpd_objc::msg_send!(string, zpd_objc::sel!("paragraphRangeForRange:"), ((selection): native::Range) => native::Range);
        let text = native::rust_string(
            zpd_objc::msg_send!(string, zpd_objc::sel!("substringWithRange:"), ((range): native::Range) => Id),
        );
        (current, text.trim_end_matches('\n').is_empty())
    });
    let selection = || {
        // SAFETY: The receiver is a live text view.
        unsafe { zpd_objc::msg_send!(object, zpd_objc::sel!("selectedRange"), () => native::Range) }
    };
    if current.is_list() && empty {
        // SAFETY: The receiver is a live rich text view.
        zpd_objc::abort_on_unwind(|| unsafe {
            TextEditor::restyle(object, selection(), |_| RichParagraph::Body, true)
        });
        return;
    }
    // SAFETY: NSTextView inserts the newline with the current typing attributes.
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSTextView"), selector, ((sender): Id) => ());
    }
    let next = match current {
        RichParagraph::Title | RichParagraph::Heading | RichParagraph::Subheading => {
            RichParagraph::Body
        }
        RichParagraph::Checklist { .. } => RichParagraph::Checklist { done: false },
        other => other,
    };
    if next != current {
        // SAFETY: The receiver is a live rich text view; the caret is in the new paragraph.
        zpd_objc::abort_on_unwind(|| unsafe {
            TextEditor::restyle(object, selection(), |_| next, true)
        });
    }
}
