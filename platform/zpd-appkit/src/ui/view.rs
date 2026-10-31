use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::OnceLock;

use crate::actor::ActorRef;
use crate::geometry::Rect;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{RcIvar, Strong};

use super::UiError;
use super::layout::{Dimension, LayoutAttribute, XAxisAnchor, YAxisAnchor};

type LayoutCallback = RefCell<Box<dyn FnMut(Rect)>>;

static LAYOUT_CALLBACK: RcIvar<LayoutCallback> = RcIvar::new(c"_zpdLayoutCallback");

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_setAssociatedObject(
        object: zpd_objc::Id,
        key: *const std::ffi::c_void,
        value: zpd_objc::Id,
        policy: usize,
    );
    fn objc_getAssociatedObject(object: zpd_objc::Id, key: *const std::ffi::c_void)
    -> zpd_objc::Id;
}

const ASSOCIATION_RETAIN_NONATOMIC: usize = 1;

/// Decides which views receive mouse input in the title bar area of a window
/// whose content extends under its title bar.
pub(crate) struct TitleBarHit;

impl TitleBarHit {
    /// Returns the associated-object key, which is the address of a private static.
    fn key() -> *const std::ffi::c_void {
        static KEY: u8 = 0;
        (&raw const KEY).cast()
    }

    /// Marks `view` and its subviews as interactive inside the title bar area.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    unsafe fn set_interactive(view: zpd_objc::Id, interactive: bool) {
        // SAFETY: Guaranteed by the caller; the runtime retains the NSNumber flag.
        unsafe {
            let value = if interactive {
                zpd_objc::msg_send!(zpd_objc::class!("NSNumber"), zpd_objc::sel!("numberWithBool:"), ((true): bool) => zpd_objc::Id)
            } else {
                zpd_objc::NIL
            };
            objc_setAssociatedObject(view, Self::key(), value, ASSOCIATION_RETAIN_NONATOMIC);
        }
    }

    /// Returns whether `view` or one of its ancestors is marked interactive.
    ///
    /// # Safety
    /// `view` must be nil or a live NSView.
    unsafe fn is_interactive(mut view: zpd_objc::Id) -> bool {
        while !view.is_null() {
            // SAFETY: Guaranteed by the caller for `view`; superviews of live views are live.
            unsafe {
                if !objc_getAssociatedObject(view, Self::key()).is_null() {
                    return true;
                }
                view = zpd_objc::msg_send!(view, zpd_objc::sel!("superview"), () => zpd_objc::Id);
            }
        }
        false
    }

    /// Returns whether `point` in `view`'s window lies in the title bar area of a
    /// window whose content extends under its title bar.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    pub(crate) unsafe fn in_title_bar(view: zpd_objc::Id, point: native::Point) -> bool {
        // SAFETY: Guaranteed by the caller; the window is checked for nil.
        unsafe {
            let window = zpd_objc::msg_send!(view, zpd_objc::sel!("window"), () => zpd_objc::Id);
            if window.is_null() {
                return false;
            }
            let mask = zpd_objc::msg_send!(window, zpd_objc::sel!("styleMask"), () => u64);
            if mask & native::NS_WINDOW_STYLE_MASK_FULL_SIZE_CONTENT_VIEW == 0 {
                return false;
            }
            let content = zpd_objc::msg_send!(window, zpd_objc::sel!("contentLayoutRect"), () => native::Rect);
            // Window coordinates grow upward, so the title bar lies above the content rect.
            point.y >= content.origin.y + content.size.height
        }
    }

    /// Returns whether input at `point` in window coordinates should reach `view`.
    ///
    /// # Safety
    /// `view` must be a live NSView.
    pub(crate) unsafe fn accepts(view: zpd_objc::Id, point: native::Point) -> bool {
        // SAFETY: Guaranteed by the caller.
        unsafe { !Self::in_title_bar(view, point) || Self::is_interactive(view) }
    }
}

/// Leaves the title bar area to the window unless the hit view is interactive there.
unsafe extern "C" fn layout_hit_test(
    object: zpd_objc::Id,
    selector: zpd_objc::Sel,
    point: native::Point,
) -> zpd_objc::Id {
    // SAFETY: NSView implements hitTest:, and `point` is in the superview's coordinates.
    unsafe {
        let hit = zpd_objc::msg_send_super!(object, zpd_objc::class!("NSView"), selector, ((point): native::Point) => zpd_objc::Id);
        if hit.is_null() {
            return hit;
        }
        let superview =
            zpd_objc::msg_send!(object, zpd_objc::sel!("superview"), () => zpd_objc::Id);
        if superview.is_null() {
            return hit;
        }
        let in_window = zpd_objc::msg_send!(superview, zpd_objc::sel!("convertPoint:toView:"), ((point): native::Point, (zpd_objc::NIL): zpd_objc::Id) => native::Point);
        if TitleBarHit::accepts(hit, in_window) {
            hit
        } else {
            zpd_objc::NIL
        }
    }
}

unsafe extern "C" fn layout(object: zpd_objc::Id, _: zpd_objc::Sel) {
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSView"), zpd_objc::sel!("layout"), () => ())
    };
    // SAFETY: This method is installed only on views with a `_zpdLayoutCallback` ivar.
    let Some(callback) = (unsafe { LAYOUT_CALLBACK.get(object) }) else {
        return;
    };
    let bounds =
        unsafe { zpd_objc::msg_send!(object, zpd_objc::sel!("bounds"), () => native::Rect) };
    zpd_objc::abort_on_unwind(|| {
        let Ok(mut invoke) = callback.try_borrow_mut() else {
            std::process::abort()
        };
        invoke(bounds.into());
    });
}

unsafe extern "C" fn layout_view_dealloc(object: zpd_objc::Id, _: zpd_objc::Sel) {
    // SAFETY: The receiver is a layout view created by `new_layout_view`.
    unsafe { release_layout_callback(object) };
    // SAFETY: NSView's dealloc completes destruction after Rust state is released.
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSView"), zpd_objc::sel!("dealloc"), () => ())
    };
}

pub(crate) unsafe fn release_layout_callback(object: zpd_objc::Id) {
    // SAFETY: The caller passes a view created by `new_layout_view`.
    drop(unsafe { LAYOUT_CALLBACK.take(object) });
}

fn layout_view_class() -> zpd_objc::Class {
    static CLASS: OnceLock<usize> = OnceLock::new();
    *CLASS.get_or_init(|| {
        zpd_objc::decl!(ZpdRustLayoutView: [zpd_objc::class!("NSView")] {
            fields { _zpdLayoutCallback: ptr }
            methods {
                "layout": "v@:" => layout,
                "hitTest:": "@@:{CGPoint=dd}" => layout_hit_test,
                "dealloc": "v@:" => layout_view_dealloc,
            }
        }) as usize
    }) as zpd_objc::Class
}

pub(crate) fn new_layout_view(frame: Rect) -> Result<Strong, UiError> {
    // SAFETY: The registered NSView subclass uses NSView's designated frame initializer.
    unsafe {
        Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(layout_view_class(), zpd_objc::sel!("alloc"), () => zpd_objc::Id), zpd_objc::sel!("initWithFrame:"), ((native::Rect::from(frame)): native::Rect) => zpd_objc::Id))
    }
    .ok_or(UiError::NativeCreationFailed)
}

pub(crate) fn set_layout_callback(
    view: ViewRef<'_>,
    callback: impl FnMut(Rect) + 'static,
) -> Result<(), UiError> {
    let callback: Box<dyn FnMut(Rect)> = Box::new(callback);
    view.with(|object| {
        // SAFETY: Layout callbacks are installed only on `new_layout_view` instances.
        unsafe { LAYOUT_CALLBACK.set(object, Rc::new(RefCell::new(callback))) }
    })
}

pub(crate) struct ViewActor {
    actor: ActorRef,
    _main_thread: PhantomData<Rc<()>>,
}
impl ViewActor {
    pub(crate) fn new(application: &Application, native: Strong) -> Self {
        let actor = application.tree().insert_root(native);
        application
            .tree()
            .add_teardown(&actor, |view| unsafe {
                zpd_objc::msg_send!(view, zpd_objc::sel!("removeFromSuperview"), () => ())
            })
            .expect("a newly inserted view actor must be live");
        Self {
            actor,
            _main_thread: PhantomData,
        }
    }
    #[cfg(feature = "wgpu")]
    pub(crate) fn from_actor(actor: ActorRef) -> Self {
        actor
            .tree_handle()
            .expect("a newly inserted view actor must have a tree")
            .add_teardown(&actor, |view| unsafe {
                zpd_objc::msg_send!(view, zpd_objc::sel!("removeFromSuperview"), () => ())
            })
            .expect("a newly inserted view actor must be live");
        Self {
            actor,
            _main_thread: PhantomData,
        }
    }
    pub(crate) fn as_view(&self) -> ViewRef<'_> {
        ViewRef { actor: &self.actor }
    }
    pub(crate) fn actor(&self) -> &ActorRef {
        &self.actor
    }
}
impl Drop for ViewActor {
    fn drop(&mut self) {
        self.actor.remove();
    }
}

pub struct View {
    inner: ViewActor,
}
impl View {
    pub fn new(application: &Application, frame: Rect) -> Result<Self, UiError> {
        // SAFETY: NSView's designated frame initializer returns a retained object.
        let native = unsafe {
            let object = zpd_objc::msg_send!(zpd_objc::class!("NSView"), zpd_objc::sel!("alloc"), () => zpd_objc::Id);
            Strong::from_retained(zpd_objc::msg_send!(object, zpd_objc::sel!("initWithFrame:"), ((native::Rect::from(frame)): native::Rect) => zpd_objc::Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        Ok(Self {
            inner: ViewActor::new(application, native),
        })
    }
}
impl AsView for View {
    fn as_view(&self) -> ViewRef<'_> {
        self.inner.as_view()
    }
}

#[derive(Clone, Copy)]
pub struct ViewRef<'view> {
    actor: &'view ActorRef,
}
impl<'view> ViewRef<'view> {
    pub(crate) fn from_actor(actor: &'view ActorRef) -> Self {
        Self { actor }
    }
    pub(crate) fn with<R>(self, operation: impl FnOnce(zpd_objc::Id) -> R) -> Result<R, UiError> {
        self.actor.with(operation).map_err(Into::into)
    }
    pub(crate) fn actor(self) -> &'view ActorRef {
        self.actor
    }
    /// Returns a weak reference to this view's Actor.
    pub fn actor_ref(self) -> ActorRef {
        self.actor.clone()
    }
    pub fn add_subview(self, child: &impl AsView) -> Result<(), UiError> {
        let child = child.as_view();
        if !self.actor.same_tree(child.actor) {
            return Err(UiError::InvalidHierarchy);
        }
        let tree = self.actor.tree_handle().ok_or(UiError::Closed)?;
        tree.validate_reparent(child.actor, self.actor)
            .map_err(UiError::from)?;
        self.with(|parent| {
            child.with(|native_child| {
                // SAFETY: Both actors are retained for this synchronous hierarchy update.
                unsafe {
                    zpd_objc::msg_send!(parent, zpd_objc::sel!("addSubview:"), ((native_child): zpd_objc::Id) => ())
                };
            })
        })??;
        tree.reparent(child.actor, self.actor)
            .map_err(UiError::from)
    }
    pub fn remove_from_superview(self) -> Result<(), UiError> {
        self.with(|view| unsafe {
            zpd_objc::msg_send!(view, zpd_objc::sel!("removeFromSuperview"), () => ())
        })?;
        self.actor.move_to_root().map_err(Into::into)
    }
    pub fn set_frame(self, frame: Rect) -> Result<(), UiError> {
        self.with(|view| unsafe {
            zpd_objc::msg_send!(view, zpd_objc::sel!("setFrame:"), ((native::Rect::from(frame)): native::Rect) => ())
        })
    }
    pub fn set_needs_layout(self, needs_layout: bool) -> Result<(), UiError> {
        self.with(|view| unsafe {
            zpd_objc::msg_send!(view, zpd_objc::sel!("setNeedsLayout:"), ((needs_layout): bool) => ())
        })
    }
    pub fn layout_subtree_if_needed(self) -> Result<(), UiError> {
        self.with(|view| unsafe {
            zpd_objc::msg_send!(view, zpd_objc::sel!("layoutSubtreeIfNeeded"), () => ())
        })
    }
    /// Returns this view's current bounds in logical points.
    pub fn bounds(self) -> Result<Rect, UiError> {
        let bounds = self.with(|view| {
            // SAFETY: NSView's bounds getter returns an NSRect for a live view.
            unsafe { zpd_objc::msg_send!(view, zpd_objc::sel!("bounds"), () => native::Rect) }
        })?;
        Ok(bounds.into())
    }
    pub fn set_identifier(self, identifier: Option<&str>) -> Result<(), UiError> {
        let identifier = identifier.map(native::nsstring);
        self.with(|view| unsafe {
            zpd_objc::msg_send!(view, zpd_objc::sel!("setAccessibilityIdentifier:"), ((identifier.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): zpd_objc::Id) => ());
        })
    }
    /// Keeps mouse input for this view and its subviews inside a title bar
    /// that the window content extends under.
    pub fn set_title_bar_interactive(self, interactive: bool) -> Result<(), UiError> {
        // SAFETY: The actor's native object is a live NSView.
        self.with(|view| unsafe { TitleBarHit::set_interactive(view, interactive) })
    }
    /// Enables or disables a control; views without `setEnabled:` ignore it.
    pub fn set_enabled(self, enabled: bool) -> Result<(), UiError> {
        self.with(|view| {
            // SAFETY: setEnabled: is sent only when the live view implements it.
            unsafe {
                if zpd_objc::msg_send!(view, zpd_objc::sel!("respondsToSelector:"), ((zpd_objc::sel!("setEnabled:")): zpd_objc::Sel) => bool)
                {
                    zpd_objc::msg_send!(view, zpd_objc::sel!("setEnabled:"), ((enabled): bool) => ());
                }
            }
        })
    }
    pub fn set_translates_autoresizing_mask_into_constraints(
        self,
        enabled: bool,
    ) -> Result<(), UiError> {
        self.with(|view| unsafe {
            zpd_objc::msg_send!(view, zpd_objc::sel!("setTranslatesAutoresizingMaskIntoConstraints:"), ((enabled): bool) => ())
        })
    }
    pub fn leading_anchor(self) -> XAxisAnchor<'view> {
        XAxisAnchor::new(self, LayoutAttribute::Leading)
    }
    pub fn trailing_anchor(self) -> XAxisAnchor<'view> {
        XAxisAnchor::new(self, LayoutAttribute::Trailing)
    }
    pub fn left_anchor(self) -> XAxisAnchor<'view> {
        XAxisAnchor::new(self, LayoutAttribute::Left)
    }
    pub fn right_anchor(self) -> XAxisAnchor<'view> {
        XAxisAnchor::new(self, LayoutAttribute::Right)
    }
    pub fn center_x_anchor(self) -> XAxisAnchor<'view> {
        XAxisAnchor::new(self, LayoutAttribute::CenterX)
    }
    pub fn top_anchor(self) -> YAxisAnchor<'view> {
        YAxisAnchor::new(self, LayoutAttribute::Top)
    }
    pub fn bottom_anchor(self) -> YAxisAnchor<'view> {
        YAxisAnchor::new(self, LayoutAttribute::Bottom)
    }
    pub fn center_y_anchor(self) -> YAxisAnchor<'view> {
        YAxisAnchor::new(self, LayoutAttribute::CenterY)
    }
    pub fn first_baseline_anchor(self) -> YAxisAnchor<'view> {
        YAxisAnchor::new(self, LayoutAttribute::FirstBaseline)
    }
    pub fn last_baseline_anchor(self) -> YAxisAnchor<'view> {
        YAxisAnchor::new(self, LayoutAttribute::LastBaseline)
    }
    pub fn width_anchor(self) -> Dimension<'view> {
        Dimension::new(self, LayoutAttribute::Width)
    }
    pub fn height_anchor(self) -> Dimension<'view> {
        Dimension::new(self, LayoutAttribute::Height)
    }
}

/// Exposes a native view handle; view operations live on [`ViewRef`].
pub trait AsView {
    fn as_view(&self) -> ViewRef<'_>;
}

impl AsView for ViewRef<'_> {
    fn as_view(&self) -> ViewRef<'_> {
        *self
    }
}
