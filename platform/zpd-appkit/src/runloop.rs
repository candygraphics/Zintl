use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::actor::{ActorRef, ActorTree, ApplicationMessage, EventRouteToken, WindowEvent};
use crate::native;
use crate::ui::{CommandSet, UiError, Window, WindowStyle};
use zpd_corefoundation::RunLoop;
use zpd_objc::{Id, Strong};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationError {
    NotMainThread,
    AlreadyInitialized,
    NotActive,
    NativeCreationFailed,
    EventHandlerAlreadyRegistered,
}
impl std::fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotMainThread => "AppKit must be initialized on the main thread",
            Self::AlreadyInitialized => "AppKit is already initialized",
            Self::NotActive => "AppKit is not active",
            Self::NativeCreationFailed => "AppKit failed to create a native object",
            Self::EventHandlerAlreadyRegistered => {
                "an AppKit window event handler is already registered"
            }
        })
    }
}
impl std::error::Error for ApplicationError {}

/// Active registration for the Application's single Window event callback.
///
/// Dropping this value unregisters the callback. Events emitted before a
/// registration exists or after it is dropped are ignored.
pub struct WindowEventRegistration<'application> {
    tree: ActorTree,
    id: u64,
    _application: PhantomData<&'application ()>,
}

impl Drop for WindowEventRegistration<'_> {
    fn drop(&mut self) {
        self.tree.clear_event_handler(self.id);
    }
}

fn shared_application() -> Id {
    // SAFETY: NSApplication's class getter returns the process singleton.
    unsafe {
        zpd_objc::msg_send!(zpd_objc::class!("NSApplication"), zpd_objc::sel!("sharedApplication"), () => zpd_objc::Id)
    }
}

/// The process's AppKit application and the Actor tree it owns.
///
/// Main-thread scheduling belongs to `messageloop-appkit`, which installs its
/// own run loop source on [`Application::run_loop`].
pub struct Application {
    run_loop: RunLoop<'static>,
    actor: ActorRef,
    tree: ActorTree,
    _main_thread: PhantomData<Rc<()>>,
}
impl Application {
    pub fn new() -> Result<Self, ApplicationError> {
        // SAFETY: pthread_main_np has no preconditions.
        if unsafe { native::pthread_main_np() } == 0 {
            return Err(ApplicationError::NotMainThread);
        }
        if INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ApplicationError::AlreadyInitialized);
        }
        // SAFETY: sharedApplication returns a live object or nil.
        let Some(app) = (unsafe { Strong::retain(shared_application()) }) else {
            INITIALIZED.store(false, Ordering::Release);
            return Err(ApplicationError::NativeCreationFailed);
        };
        // SAFETY: setActivationPolicy: takes an NSInteger policy on a live NSApplication.
        unsafe {
            zpd_objc::msg_send!(app.as_ptr(), zpd_objc::sel!("setActivationPolicy:"), ((native::NS_APPLICATION_ACTIVATION_POLICY_REGULAR): i64) => ())
        };
        // Each Application gets a fresh tree, so ActorRefs from an earlier one stay dropped.
        let tree = ActorTree::new(app);
        Ok(Self {
            run_loop: RunLoop::current(),
            actor: tree.root(),
            tree,
            _main_thread: PhantomData,
        })
    }
    pub fn run_loop(&self) -> RunLoop<'_> {
        self.run_loop
    }
    pub fn create_window(&self) -> Result<Window<'_>, UiError> {
        self.create_window_with_event_route(None)
    }

    /// Creates a Window whose Actor carries an opaque event route before the
    /// `Created` event is emitted.
    pub fn create_window_with_event_route(
        &self,
        route: Option<EventRouteToken>,
    ) -> Result<Window<'_>, UiError> {
        Window::new(self, route, WindowStyle::Normal)
    }

    /// Creates a Window of `style` whose Actor carries an opaque event route.
    pub fn create_styled_window(
        &self,
        route: Option<EventRouteToken>,
        style: WindowStyle,
    ) -> Result<Window<'_>, UiError> {
        Window::new(self, route, style)
    }
    /// Registers the sole semantic Window event callback for this Application.
    ///
    /// The callback runs synchronously at the AppKit event source. Consumers
    /// should enqueue work instead of invoking re-entrant UI processing.
    pub fn on(
        &self,
        callback: impl FnMut(WindowEvent) + 'static,
    ) -> Result<WindowEventRegistration<'_>, ApplicationError> {
        let id = self
            .tree
            .set_event_handler(callback)
            .map_err(|_| ApplicationError::EventHandlerAlreadyRegistered)?;
        Ok(WindowEventRegistration {
            tree: self.tree.clone(),
            id,
            _application: PhantomData,
        })
    }
    pub fn set_commands<F>(&self, commands: &CommandSet, callback: F) -> Result<(), UiError>
    where
        F: FnMut(&str) + 'static,
    {
        crate::ui::commands::install(self, commands, callback)
    }
    pub fn run(&self) -> Result<(), ApplicationError> {
        self.actor
            .send(ApplicationMessage::Run)
            .map_err(|_| ApplicationError::NotActive)
    }
    pub fn stop(&self) -> Result<(), ApplicationError> {
        self.actor
            .send(ApplicationMessage::Stop)
            .map_err(|_| ApplicationError::NotActive)
    }
    pub fn actor_ref(&self) -> ActorRef {
        self.actor.clone()
    }
    pub(crate) fn tree(&self) -> &ActorTree {
        &self.tree
    }
}
impl Drop for Application {
    fn drop(&mut self) {
        // SAFETY: Clearing the menu detaches unretained action targets before their owners drop.
        unsafe {
            let _ = self
                .actor
                .with(|app| zpd_objc::msg_send!(app, zpd_objc::sel!("setMainMenu:"), ((zpd_objc::NIL): zpd_objc::Id) => ()));
        }
        self.tree.clear();
        INITIALIZED.store(false, Ordering::Release);
    }
}

pub(crate) fn send_application_message(
    actor: &ActorRef,
    message: ApplicationMessage,
) -> Result<(), crate::actor::ActorError> {
    actor.with(|app| unsafe {
        match message {
            ApplicationMessage::Run => {
                zpd_objc::msg_send!(app, zpd_objc::sel!("activateIgnoringOtherApps:"), ((true): bool) => ());
                zpd_objc::msg_send!(app, zpd_objc::sel!("run"), () => ());
            }
            ApplicationMessage::Stop => {
                zpd_objc::msg_send!(app, zpd_objc::sel!("stop:"), ((zpd_objc::NIL): zpd_objc::Id) => ());
                let event = zpd_objc::msg_send!(zpd_objc::class!("NSEvent"), zpd_objc::sel!("otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:"), ((native::NS_EVENT_TYPE_APPLICATION_DEFINED): i64, (native::Point::default()): native::Point, (0): u64, (0.0): f64, (0): i64, (zpd_objc::NIL): zpd_objc::Id, (0): i16, (0): i64, (0): i64) => zpd_objc::Id);
                if !event.is_null() {
                    zpd_objc::msg_send!(app, zpd_objc::sel!("postEvent:atStart:"), ((event): zpd_objc::Id, (false): bool) => ());
                }
                RunLoop::main().stop();
                RunLoop::main().wake();
            }
        }
    })
}
