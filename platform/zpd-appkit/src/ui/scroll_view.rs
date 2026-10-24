use std::sync::OnceLock;

use crate::actor::ActorRef;
use crate::geometry::Rect;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, Strong};

use super::UiError;
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

/// An `NSScrollView` whose document view uses top-left coordinates.
pub struct ScrollView {
    view: ViewActor,
    document: ActorRef,
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
        Ok(Self { view, document })
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
