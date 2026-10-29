use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::OnceLock;

use crate::actor::{ActorRef, ActorRollback, EventRouteToken, WindowEventKind};
#[cfg(feature = "wgpu")]
use crate::geometry::PhysicalSize;
use crate::geometry::Rect;
use crate::native;
use crate::runloop::Application;
use zpd_objc::{Id, RcIvar, Strong};

use super::UiError;
use super::sidebar::{self, Sidebar, SidebarNative};
#[cfg(feature = "wgpu")]
use super::view::AsView;
use super::view::{self, ViewRef};

struct WindowState {
    actor: RefCell<Option<ActorRef>>,
}
impl WindowState {
    fn resized(&self) {
        let actor = self.actor.borrow().clone();
        if let Some(actor) = actor
            && let Some(tree) = actor.tree_handle()
        {
            tree.emit(&actor, WindowEventKind::DidResize);
        }
    }

    fn close(&self) {
        // Taking the actor makes repeated close notifications no-ops.
        let actor = self.actor.borrow_mut().take();
        if let Some(actor) = actor {
            if let Some(tree) = actor.tree_handle() {
                tree.emit(&actor, WindowEventKind::WillClose);
                tree.emit(&actor, WindowEventKind::DidClose);
            }
            actor.remove();
        }
    }
}

static WINDOW_STATE: RcIvar<WindowState> = RcIvar::new(c"_zpdWindowState");

unsafe extern "C" fn window_will_close(object: Id, _: zpd_objc::Sel, _: Id) {
    // SAFETY: The callback receiver is live on entry; retaining it keeps the delegate alive if
    // closing removes the owning Actor subtree synchronously.
    let _receiver = unsafe { Strong::retain(object) };
    // SAFETY: This callback is installed only on delegates with a `_zpdWindowState` ivar.
    if let Some(state) = unsafe { WINDOW_STATE.get(object) } {
        zpd_objc::abort_on_unwind(|| state.close());
    }
}
unsafe extern "C" fn window_did_resize(object: Id, _: zpd_objc::Sel, _: Id) {
    // SAFETY: The callback receiver is live on entry; retaining it keeps the delegate alive if
    // resize handling synchronously removes the Window subtree.
    let _receiver = unsafe { Strong::retain(object) };
    // SAFETY: This callback is installed only on delegates with a `_zpdWindowState` ivar.
    if let Some(state) = unsafe { WINDOW_STATE.get(object) } {
        zpd_objc::abort_on_unwind(|| state.resized());
    }
}
unsafe extern "C" fn delegate_dealloc(object: Id, _: zpd_objc::Sel) {
    // SAFETY: The receiver is a window delegate with a `_zpdWindowState` ivar.
    drop(unsafe { WINDOW_STATE.take(object) });
    // SAFETY: NSObject's dealloc completes destruction after Rust state is released.
    unsafe {
        zpd_objc::msg_send_super!(object, zpd_objc::class!("NSObject"), zpd_objc::sel!("dealloc"), () => ())
    };
}

fn window_delegate_class() -> zpd_objc::Class {
    static CLASS: OnceLock<usize> = OnceLock::new();
    *CLASS.get_or_init(|| {
        zpd_objc::decl!(ZpdRustWindowDelegate: [zpd_objc::class!("NSObject")] {
            fields { _zpdWindowState: ptr }
            methods {
                "windowWillClose:": "v@:@" => window_will_close,
                "windowDidResize:": "v@:@" => window_did_resize,
                "dealloc": "v@:" => delegate_dealloc,
            }
        }) as usize
    }) as zpd_objc::Class
}

/// Selects the native window class and chrome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowStyle {
    /// A standard titled, resizable `NSWindow`.
    #[default]
    Normal,
    /// A floating utility `NSPanel` that hides while the app is inactive.
    Panel,
}

pub struct Window<'application> {
    actor: ActorRef,
    content: ActorRef,
    content_controller: ActorRef,
    sidebar: RefCell<Option<SidebarNative>>,
    sidebar_toolbar: RefCell<Option<super::sidebar_toolbar::SidebarToolbar>>,
    _application: PhantomData<&'application Application>,
    _main_thread: PhantomData<Rc<()>>,
}
impl<'application> Window<'application> {
    pub(crate) fn new(
        application: &'application Application,
        event_route: Option<EventRouteToken>,
        style: WindowStyle,
    ) -> Result<Self, UiError> {
        let state = Rc::new(WindowState {
            actor: RefCell::new(None),
        });
        let frame = native::Rect {
            origin: native::Point { x: 0.0, y: 0.0 },
            size: native::Size {
                width: 480.0,
                height: 300.0,
            },
        };
        let (class, style_mask) = match style {
            WindowStyle::Normal => (
                zpd_objc::class!("NSWindow"),
                native::NS_WINDOW_STYLE_MASK_TITLED
                    | native::NS_WINDOW_STYLE_MASK_CLOSABLE
                    | native::NS_WINDOW_STYLE_MASK_MINIATURIZABLE
                    | native::NS_WINDOW_STYLE_MASK_RESIZABLE,
            ),
            WindowStyle::Panel => (
                zpd_objc::class!("NSPanel"),
                native::NS_WINDOW_STYLE_MASK_TITLED
                    | native::NS_WINDOW_STYLE_MASK_CLOSABLE
                    | native::NS_WINDOW_STYLE_MASK_RESIZABLE
                    | native::NS_WINDOW_STYLE_MASK_UTILITY_WINDOW,
            ),
        };
        // SAFETY: This is NSWindow's arm64 designated initializer signature, shared by NSPanel.
        let native_window = unsafe {
            Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(class, zpd_objc::sel!("alloc"), () => zpd_objc::Id), zpd_objc::sel!("initWithContentRect:styleMask:backing:defer:"), ((frame): native::Rect, (style_mask): u64, (native::NS_BACKING_STORE_BUFFERED): u64, (false): bool) => zpd_objc::Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        if style == WindowStyle::Panel {
            // SAFETY: The retained object is an NSPanel.
            unsafe {
                zpd_objc::msg_send!(native_window.as_ptr(), zpd_objc::sel!("setFloatingPanel:"), ((true): bool) => ());
                zpd_objc::msg_send!(native_window.as_ptr(), zpd_objc::sel!("setHidesOnDeactivate:"), ((true): bool) => ());
            }
        }
        unsafe {
            zpd_objc::msg_send!(native_window.as_ptr(), zpd_objc::sel!("setReleasedWhenClosed:"), ((false): bool) => ())
        };
        let actor = application.tree().insert_window(native_window);
        actor.set_event_route(event_route).map_err(UiError::from)?;
        let mut rollback = ActorRollback::new(actor.clone());
        *state.actor.borrow_mut() = Some(actor.clone());
        application
            .tree()
            .add_teardown(&actor, |window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("setDelegate:"), ((zpd_objc::NIL): zpd_objc::Id) => ());
                zpd_objc::msg_send!(window, zpd_objc::sel!("setContentViewController:"), ((zpd_objc::NIL): zpd_objc::Id) => ());
            })
            .map_err(UiError::from)?;

        let controller = native::alloc_init(zpd_objc::class!("NSViewController"));
        let view = view::new_layout_view(frame.into())?;
        unsafe {
            zpd_objc::msg_send!(controller.as_ptr(), zpd_objc::sel!("setView:"), ((view.as_ptr()): zpd_objc::Id) => ());
            actor
                .with(|window| {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("setContentViewController:"), ((controller.as_ptr()): zpd_objc::Id) => ())
                })
                .map_err(UiError::from)?;
        }
        let content = application
            .tree()
            .insert_child(&actor, view)
            .map_err(UiError::from)?;
        application
            .tree()
            .add_teardown(&content, |view| unsafe {
                view::release_layout_callback(view)
            })
            .map_err(UiError::from)?;
        let content_controller = application
            .tree()
            .replace_owned(&actor, "content-controller", controller)
            .map_err(UiError::from)?;
        let native_delegate = native::alloc_init(window_delegate_class());
        // SAFETY: The registered delegate class declares the `_zpdWindowState` ivar.
        unsafe { WINDOW_STATE.set(native_delegate.as_ptr(), state) };
        actor
            .with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("setDelegate:"), ((native_delegate.as_ptr()): zpd_objc::Id) => ())
            })
            .map_err(UiError::from)?;
        let delegate_actor = application
            .tree()
            .replace_owned(&actor, "delegate", native_delegate)
            .map_err(UiError::from)?;
        application
            .tree()
            .add_teardown(&delegate_actor, |delegate| {
                // SAFETY: The delegate Actor owns a window delegate created above.
                drop(unsafe { WINDOW_STATE.take(delegate) });
            })
            .map_err(UiError::from)?;
        application.tree().emit(&actor, WindowEventKind::Created);
        rollback.disarm();
        Ok(Self {
            actor,
            content,
            content_controller,
            sidebar: RefCell::new(None),
            sidebar_toolbar: RefCell::new(None),
            _application: PhantomData,
            _main_thread: PhantomData,
        })
    }
    /// Checks liveness for operations that do not message the window itself.
    fn ensure_open(&self) -> Result<(), UiError> {
        if self.actor.is_alive() {
            Ok(())
        } else {
            Err(UiError::Closed)
        }
    }
    /// Returns the main screen's top edge for flipping top-left window coordinates.
    fn screen_top() -> f64 {
        // SAFETY: NSScreen's class getter may return nil, which is handled below.
        let screen = unsafe {
            zpd_objc::msg_send!(zpd_objc::class!("NSScreen"), zpd_objc::sel!("mainScreen"), () => zpd_objc::Id)
        };
        if screen.is_null() {
            return 0.0;
        }
        // SAFETY: A non-nil NSScreen returns its frame as an NSRect.
        let frame =
            unsafe { zpd_objc::msg_send!(screen, zpd_objc::sel!("frame"), () => native::Rect) };
        frame.origin.y + frame.size.height
    }
    pub fn is_closed(&self) -> bool {
        !self.actor.is_alive()
    }
    /// Returns a weak reference to this window's Actor.
    pub fn actor_ref(&self) -> ActorRef {
        self.actor.clone()
    }
    pub fn show(&self) -> Result<(), UiError> {
        self.actor
            .with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("makeKeyAndOrderFront:"), ((zpd_objc::NIL): zpd_objc::Id) => ())
            })
            .map_err(Into::into)
    }
    /// Returns whether a sheet is currently attached to this window.
    pub fn has_attached_sheet(&self) -> bool {
        self.actor
            .with(|window| {
                // SAFETY: The live actor is an NSWindow.
                let sheet = unsafe {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("attachedSheet"), () => zpd_objc::Id)
                };
                !sheet.is_null()
            })
            .unwrap_or(false)
    }
    pub fn set_title(&self, title: &str) -> Result<(), UiError> {
        let title = native::nsstring(title);
        self.actor
            .with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("setTitle:"), ((title.as_ptr()): zpd_objc::Id) => ())
            })
            .map_err(Into::into)
    }
    pub fn set_identifier(&self, identifier: Option<&str>) -> Result<(), UiError> {
        let identifier = identifier.map(native::nsstring);
        self.actor
            .with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("setAccessibilityIdentifier:"), ((identifier.as_ref().map_or(zpd_objc::NIL, Strong::as_ptr)): zpd_objc::Id) => ())
            })
            .map_err(Into::into)
    }
    pub fn set_full_size_content_view(&self, enabled: bool) -> Result<(), UiError> {
        self.actor
            .with(|window| {
                // SAFETY: The live actor is an NSWindow and both selectors use NSUInteger masks.
                unsafe {
                    let mut style_mask =
                        zpd_objc::msg_send!(window, zpd_objc::sel!("styleMask"), () => u64);
                    let is_enabled = style_mask
                        & native::NS_WINDOW_STYLE_MASK_FULL_SIZE_CONTENT_VIEW
                        != 0;
                    if enabled != is_enabled {
                        if enabled {
                            style_mask |= native::NS_WINDOW_STYLE_MASK_FULL_SIZE_CONTENT_VIEW;
                        } else {
                            style_mask &= !native::NS_WINDOW_STYLE_MASK_FULL_SIZE_CONTENT_VIEW;
                        }
                        zpd_objc::msg_send!(window, zpd_objc::sel!("setStyleMask:"), ((style_mask): u64) => ())
                    }
                }
            })
            .map_err(Into::into)
    }
    pub fn set_bounds(&self, bounds: Rect) -> Result<(), UiError> {
        let frame = native::Rect::from(Rect {
            y: Self::screen_top() - bounds.y - bounds.height,
            ..bounds
        });
        self.actor
            .with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("setFrame:display:"), ((frame): native::Rect, (true): bool) => ())
            })
            .map_err(Into::into)
    }
    pub fn set_size(&self, width: f64, height: f64) -> Result<(), UiError> {
        let mut frame = self
            .actor
            .with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("frame"), () => native::Rect)
            })
            .map_err(UiError::from)?;
        frame.size = native::Size { width, height };
        self.actor
            .with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("setFrame:display:"), ((frame): native::Rect, (true): bool) => ())
            })
            .map_err(Into::into)
    }
    pub fn set_position(&self, x: f64, y: f64) -> Result<(), UiError> {
        let top = Self::screen_top() - y;
        self.actor
            .with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("setFrameTopLeftPoint:"), ((native::Point { x, y: top }): native::Point) => ())
            })
            .map_err(Into::into)
    }
    pub fn content_view(&self) -> Result<ViewRef<'_>, UiError> {
        self.ensure_open()?;
        Ok(ViewRef::from_actor(&self.content))
    }
    pub fn set_content_layout_handler(
        &self,
        callback: impl FnMut(Rect) + 'static,
    ) -> Result<(), UiError> {
        view::set_layout_callback(ViewRef::from_actor(&self.content), callback)
    }
    pub fn set_sidebar<F>(&self, sidebar: &Sidebar, mut callback: F) -> Result<(), UiError>
    where
        F: FnMut(&str) + 'static,
    {
        let frame = self.actor.with(|window| {
            // SAFETY: The live actor is an NSWindow and `frame` returns an NSRect by value.
            unsafe { zpd_objc::msg_send!(window, zpd_objc::sel!("frame"), () => native::Rect) }
        })?;
        let actor = self.actor.clone();
        let collapsed = self
            .sidebar
            .borrow()
            .as_ref()
            .is_some_and(SidebarNative::is_collapsed);
        let native = sidebar::install(
            &self.actor,
            &self.content_controller,
            sidebar,
            collapsed,
            move |id| {
                if let Some(tree) = actor.tree_handle() {
                    tree.emit(
                        &actor,
                        WindowEventKind::SidebarSelectionChanged { id: id.into() },
                    );
                }
                callback(id);
            },
        )?;
        self.actor
            .with(|window| {
                // SAFETY: Restoring the captured NSRect keeps controller replacement from
                // changing the user's current window position or size.
                unsafe {
                    zpd_objc::msg_send!(window, zpd_objc::sel!("setFrame:display:"), ((frame): native::Rect, (true): bool) => ())
                }
            })?;
        *self.sidebar.borrow_mut() = Some(native);
        if self.sidebar_toolbar.borrow().is_none() {
            *self.sidebar_toolbar.borrow_mut() = Some(
                super::sidebar_toolbar::SidebarToolbar::install(&self.actor)?,
            );
        }
        Ok(())
    }
    /// Applies `sidebar`, keeping native views when only items, selection, or search text change.
    pub fn update_sidebar(&self, sidebar: &Sidebar) -> Result<(), UiError> {
        self.ensure_open()?;
        if let Some(native) = self.sidebar.borrow().as_ref()
            && native.can_update(sidebar)
        {
            return native.update(sidebar);
        }
        self.set_sidebar(sidebar, |_| {})
    }
    pub fn clear_sidebar(&self) -> Result<(), UiError> {
        self.ensure_open()?;
        if let Some(sidebar) = self.sidebar.borrow_mut().take() {
            sidebar.clear(&self.actor)?;
        }
        if let Some(toolbar) = self.sidebar_toolbar.borrow_mut().take() {
            toolbar.clear(&self.actor)?;
        }
        Ok(())
    }
    #[cfg(feature = "wgpu")]
    pub fn create_wgpu_surface(&self, rect: Rect) -> Result<WgpuSurface<'application>, UiError> {
        self.ensure_open()?;
        WgpuSurface::new(&self.content, rect)
    }
}
impl Drop for Window<'_> {
    fn drop(&mut self) {
        self.sidebar.borrow_mut().take();
        if self.actor.is_alive() {
            let _ = self.actor.with(|window| unsafe {
                zpd_objc::msg_send!(window, zpd_objc::sel!("close"), () => ())
            });
        }
        self.actor.remove();
    }
}

#[cfg(feature = "wgpu")]
#[derive(Clone, Copy)]
pub struct MetalLayer<'surface> {
    actor: &'surface ActorRef,
}
#[cfg(feature = "wgpu")]
impl MetalLayer<'_> {
    pub fn as_ptr(self) -> Result<*mut std::ffi::c_void, UiError> {
        self.actor.with(|id| id).map_err(UiError::from)
    }
}

#[cfg(feature = "wgpu")]
pub struct WgpuSurface<'application> {
    view: super::view::ViewActor,
    layer: ActorRef,
    _application: PhantomData<&'application Application>,
}
#[cfg(feature = "wgpu")]
impl<'application> WgpuSurface<'application> {
    fn new(content: &ActorRef, rect: Rect) -> Result<Self, UiError> {
        let native_view = unsafe {
            Strong::from_retained(zpd_objc::msg_send!(zpd_objc::msg_send!(zpd_objc::class!("NSView"), zpd_objc::sel!("alloc"), () => zpd_objc::Id), zpd_objc::sel!("initWithFrame:"), ((native::Rect::from(rect)): native::Rect) => zpd_objc::Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        let tree = content.tree_handle().ok_or(UiError::Closed)?;
        let view_actor = tree.insert_child(content, native_view)?;
        let layer_native = unsafe {
            Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("CAMetalLayer"), zpd_objc::sel!("layer"), () => zpd_objc::Id))
        }
        .ok_or(UiError::NativeCreationFailed)?;
        let layer = tree
            .insert_child(&view_actor, layer_native)
            .map_err(UiError::from)?;
        view_actor
            .with(|view| {
                layer.with(|layer| unsafe {
                    zpd_objc::msg_send!(view, zpd_objc::sel!("setWantsLayer:"), ((true): bool) => ());
                    zpd_objc::msg_send!(view, zpd_objc::sel!("setLayer:"), ((layer): zpd_objc::Id) => ());
                })
            })
            .map_err(UiError::from)?
            .map_err(UiError::from)?;
        content
            .with(|parent| {
                view_actor.with(|view| unsafe {
                    zpd_objc::msg_send!(parent, zpd_objc::sel!("addSubview:"), ((view): zpd_objc::Id) => ())
                })
            })
            .map_err(UiError::from)?
            .map_err(UiError::from)?;
        let surface = Self {
            view: super::view::ViewActor::from_actor(view_actor),
            layer,
            _application: PhantomData,
        };
        surface.update_drawable_size()?;
        Ok(surface)
    }
    pub fn set_rect(&self, rect: Rect) -> Result<(), UiError> {
        self.view.as_view().set_frame(rect)?;
        self.update_drawable_size()
    }
    pub fn drawable_size(&self) -> Result<PhysicalSize, UiError> {
        self.update_drawable_size()?;
        let size = self
            .layer
            .with(|layer| unsafe {
                zpd_objc::msg_send!(layer, zpd_objc::sel!("drawableSize"), () => native::Size)
            })
            .map_err(UiError::from)?;
        Ok(PhysicalSize::new(
            size.width.max(0.0).round() as u32,
            size.height.max(0.0).round() as u32,
        ))
    }
    fn update_drawable_size(&self) -> Result<(), UiError> {
        let bounds = self
            .view
            .actor()
            .with(|view| unsafe {
                zpd_objc::msg_send!(view, zpd_objc::sel!("bounds"), () => native::Rect)
            })
            .map_err(UiError::from)?;
        let screen = unsafe {
            zpd_objc::msg_send!(zpd_objc::class!("NSScreen"), zpd_objc::sel!("mainScreen"), () => zpd_objc::Id)
        };
        let scale = if screen.is_null() {
            1.0
        } else {
            unsafe { zpd_objc::msg_send!(screen, zpd_objc::sel!("backingScaleFactor"), () => f64) }
        };
        self.layer
            .with(|layer| unsafe {
                zpd_objc::msg_send!(layer, zpd_objc::sel!("setContentsScale:"), ((scale): f64) => ());
                zpd_objc::msg_send!(layer, zpd_objc::sel!("setDrawableSize:"), ((native::Size {
                        width: bounds.size.width * scale,
                        height: bounds.size.height * scale,
                    }): native::Size) => ());
            })
            .map_err(Into::into)
    }
    pub fn metal_layer(&self) -> Result<MetalLayer<'_>, UiError> {
        self.layer.with(|_| ()).map_err(UiError::from)?;
        Ok(MetalLayer { actor: &self.layer })
    }
}
#[cfg(feature = "wgpu")]
impl AsView for WgpuSurface<'_> {
    fn as_view(&self) -> ViewRef<'_> {
        self.view.as_view()
    }
}
