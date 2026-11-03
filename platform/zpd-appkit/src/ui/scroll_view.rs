use std::cell::Cell;
use std::rc::Rc;
use std::sync::OnceLock;

use crate::actor::{ActorRef, WindowEventKind};
use crate::geometry::Rect;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, Strong};

use super::UiError;
use super::callback;
use super::view::{AsView, ViewActor, ViewRef};

unsafe extern "C" fn is_flipped(_: Id, _: zpd_objc::Sel) -> bool {
    true
}

fn flipped_view_class() -> zpd_objc::Class {
    static CLASS: OnceLock<usize> = OnceLock::new();
    *CLASS.get_or_init(|| {
        zpd_objc::decl!(ZpdRustFlippedView: [zpd_objc::class!("NSView")] {
            methods {
                "isFlipped": "B@:" => is_flipped,
            }
        }) as usize
    }) as zpd_objc::Class
}

/// Scroll directions enabled on a [`ScrollView`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollAxes {
    pub horizontal: bool,
    pub vertical: bool,
}

/// Where [`ScrollView::reveal`] places a view in the visible area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAlignment {
    /// Scrolls only as far as needed to show the whole view.
    #[default]
    Nearest,
    Start,
    Center,
    End,
}

impl ScrollAlignment {
    /// Returns the new offset along one axis for an item at `start..start + length`.
    fn offset(self, current: f64, visible: f64, start: f64, length: f64) -> f64 {
        match self {
            Self::Start => start,
            Self::Center => start + length / 2.0 - visible / 2.0,
            Self::End => start + length - visible,
            Self::Nearest if start < current => start,
            Self::Nearest if start + length > current + visible => start + length - visible,
            Self::Nearest => current,
        }
    }
}

/// An `NSScrollView` whose document view uses top-left coordinates.
pub struct ScrollView {
    view: ViewActor,
    document: ActorRef,
    // Programmatic scrolling must not be reported as a user change.
    quiet: Rc<Cell<bool>>,
}
impl ScrollView {
    pub fn new(application: &Application) -> Result<Self, UiError> {
        // SAFETY: NSScrollView and the registered NSView subclass implement alloc/init.
        let (scroll, document) = unsafe {
            let scroll = Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSScrollView"), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("init"), () => Id))
                .ok_or(UiError::NativeCreationFailed)?;
            let document = Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(flipped_view_class(), zpd_objc::sel!("alloc"), () => Id), zpd_objc::sel!("init"), () => Id))
                .ok_or(UiError::NativeCreationFailed)?;
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDrawsBackground:"), ((false): bool) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setAutohidesScrollers:"), ((true): bool) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setHasVerticalScroller:"), ((true): bool) => ());
            zpd_objc::msg_send!(scroll.as_ptr(), zpd_objc::sel!("setDocumentView:"), ((document.as_ptr()): Id) => ());
            (scroll, document)
        };
        let view = ViewActor::new(application, scroll);
        let tree = view.actor().tree_handle().ok_or(UiError::Closed)?;
        let document = tree.insert_child(view.actor(), document)?;
        let quiet = Rc::new(Cell::new(false));
        let silenced = quiet.clone();
        let emit = callback::emitter(view.actor());
        let mut last = native::Point::default();
        // SAFETY: The live view is an NSScrollView that retains its clip view.
        let clip = view.as_view().with(|scroll| unsafe {
            let clip = zpd_objc::msg_send!(scroll, zpd_objc::sel!("contentView"), () => Id);
            zpd_objc::msg_send!(clip, zpd_objc::sel!("setPostsBoundsChangedNotifications:"), ((true): bool) => ());
            clip
        })?;
        callback::install_observer(
            view.actor(),
            "NSViewBoundsDidChangeNotification",
            clip,
            move |notification| {
                // SAFETY: Bounds notifications carry the live NSClipView as their object.
                let origin = unsafe {
                    let clip =
                        zpd_objc::msg_send!(notification, zpd_objc::sel!("object"), () => Id);
                    zpd_objc::msg_send!(clip, zpd_objc::sel!("bounds"), () => native::Rect).origin
                };
                // Resizing also changes the bounds; only report a moved origin.
                if (origin.x, origin.y) == (last.x, last.y) {
                    return;
                }
                last = origin;
                if !silenced.get() {
                    emit(WindowEventKind::Scrolled {
                        x: origin.x,
                        y: origin.y,
                    });
                }
            },
        )?;
        Ok(Self {
            view,
            document,
            quiet,
        })
    }
    pub fn set_axes(&self, axes: ScrollAxes) -> Result<(), UiError> {
        self.view.as_view().with(|scroll| {
            // SAFETY: The live view is an NSScrollView.
            unsafe {
                zpd_objc::msg_send!(scroll, zpd_objc::sel!("setHasHorizontalScroller:"), ((axes.horizontal): bool) => ());
                zpd_objc::msg_send!(scroll, zpd_objc::sel!("setHasVerticalScroller:"), ((axes.vertical): bool) => ());
            }
        })
    }
    /// Returns the view that hosts scrolled content.
    pub fn document_view(&self) -> ViewRef<'_> {
        ViewRef::from_actor(&self.document)
    }
    /// Returns the visible size available to the document view.
    pub fn content_size(&self) -> Result<(f64, f64), UiError> {
        let size = self.view.as_view().with(|scroll| {
            // SAFETY: The live view is an NSScrollView returning an NSSize.
            unsafe {
                zpd_objc::msg_send!(scroll, zpd_objc::sel!("contentSize"), () => native::Size)
            }
        })?;
        Ok((size.width, size.height))
    }
    /// Scrolls `(x, y)` to the top-left corner, clamped to the document.
    ///
    /// This is not reported as a [`WindowEventKind::Scrolled`] event.
    pub fn scroll_to(&self, x: f64, y: f64) -> Result<(), UiError> {
        self.quiet.set(true);
        let result = self.view.as_view().with(|scroll| {
            // SAFETY: The live view is an NSScrollView and its clip view takes NSRect/NSPoint values.
            unsafe {
                let clip = zpd_objc::msg_send!(scroll, zpd_objc::sel!("contentView"), () => Id);
                let mut bounds = zpd_objc::msg_send!(clip, zpd_objc::sel!("bounds"), () => native::Rect);
                bounds.origin = native::Point { x, y };
                let bounds = zpd_objc::msg_send!(clip, zpd_objc::sel!("constrainBoundsRect:"), ((bounds): native::Rect) => native::Rect);
                zpd_objc::msg_send!(clip, zpd_objc::sel!("scrollToPoint:"), ((bounds.origin): native::Point) => ());
                zpd_objc::msg_send!(scroll, zpd_objc::sel!("reflectScrolledClipView:"), ((clip): Id) => ());
            }
        });
        self.quiet.set(false);
        result
    }
    /// Scrolls the descendant whose accessibility identifier is `identifier` into view.
    ///
    /// Returns `false` when no such view is in the document. The scroll is
    /// reported as a user scroll so bound offsets follow it.
    pub fn reveal(&self, identifier: &str, alignment: ScrollAlignment) -> Result<bool, UiError> {
        let document = self.document.with(|document| document)?;
        // SAFETY: The document view and its subviews are live NSViews on the main thread.
        let Some(frame) = (unsafe { Self::find(document, identifier) }) else {
            return Ok(false);
        };
        self.view.as_view().with(|scroll| {
            // SAFETY: The live view is an NSScrollView and its clip view takes NSRect/NSPoint values.
            unsafe {
                let clip = zpd_objc::msg_send!(scroll, zpd_objc::sel!("contentView"), () => Id);
                let mut bounds = zpd_objc::msg_send!(clip, zpd_objc::sel!("bounds"), () => native::Rect);
                bounds.origin.x = alignment.offset(bounds.origin.x, bounds.size.width, frame.origin.x, frame.size.width);
                bounds.origin.y = alignment.offset(bounds.origin.y, bounds.size.height, frame.origin.y, frame.size.height);
                let bounds = zpd_objc::msg_send!(clip, zpd_objc::sel!("constrainBoundsRect:"), ((bounds): native::Rect) => native::Rect);
                zpd_objc::msg_send!(clip, zpd_objc::sel!("scrollToPoint:"), ((bounds.origin): native::Point) => ());
                zpd_objc::msg_send!(scroll, zpd_objc::sel!("reflectScrolledClipView:"), ((clip): Id) => ());
            }
        })?;
        Ok(true)
    }
    /// Returns the frame in `document` coordinates of its first descendant named `identifier`.
    ///
    /// # Safety
    /// `document` must be a live NSView.
    unsafe fn find(document: Id, identifier: &str) -> Option<native::Rect> {
        let mut pending = vec![document];
        while let Some(view) = pending.pop() {
            // SAFETY: Guaranteed by the caller for the root; subviews are live while their parent is.
            unsafe {
                let name = zpd_objc::msg_send!(view, zpd_objc::sel!("accessibilityIdentifier"), () => Id);
                if view != document && !name.is_null() && native::rust_string(name) == identifier {
                    let bounds = zpd_objc::msg_send!(view, zpd_objc::sel!("bounds"), () => native::Rect);
                    return Some(zpd_objc::msg_send!(view, zpd_objc::sel!("convertRect:toView:"), ((bounds): native::Rect, (document): Id) => native::Rect));
                }
                let subviews = zpd_objc::msg_send!(view, zpd_objc::sel!("subviews"), () => Id);
                let count = zpd_objc::msg_send!(subviews, zpd_objc::sel!("count"), () => usize);
                for index in (0..count).rev() {
                    pending.push(zpd_objc::msg_send!(subviews, zpd_objc::sel!("objectAtIndex:"), ((index): usize) => Id));
                }
            }
        }
        None
    }
    /// Resizes the scrollable document area.
    pub fn set_document_size(&self, width: f64, height: f64) -> Result<(), UiError> {
        self.document_view()
            .set_frame(Rect::new(0.0, 0.0, width, height))
    }
}
impl AsView for ScrollView {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}
