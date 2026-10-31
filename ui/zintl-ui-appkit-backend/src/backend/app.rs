use std::cell::RefCell;
use std::rc::Rc;

use messageloop_appkit::{Context as MessageContext, MessageLoopAppkit, MessageLoopHandler};
use zintl_ui::composer::Composer;
use zintl_ui::event::EventRouteId;
use zpd_appkit::actor::{WindowEvent, WindowEventKind};
use zpd_appkit::runloop::Application;

use super::{AppError, AppKitBackend};
use crate::convert::Shared;
use crate::{Event, MainTask, RenderNode};

pub(super) enum Message {
    Window(WindowEvent),
    PerformMain(MainTask),
    /// A custom main menu item was chosen.
    Command(String),
}

impl From<WindowEvent> for Message {
    fn from(event: WindowEvent) -> Self {
        Self::Window(event)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WindowEventAction {
    Synchronize,
    WaitForClose,
    /// The window is gone; synchronize and end the app if it was the last one.
    Closed,
}

impl WindowEventAction {
    pub(super) fn of(kind: &WindowEventKind) -> Self {
        match kind {
            WindowEventKind::WillClose => Self::WaitForClose,
            WindowEventKind::DidClose => Self::Closed,
            _ => Self::Synchronize,
        }
    }
}

struct AppHandler<'application> {
    application: &'application Application,
    composer: Composer<RenderNode, AppKitBackend>,
    error: Rc<RefCell<Option<AppError>>>,
}

impl AppHandler<'_> {
    fn synchronize(&mut self, cx: &MessageContext<'_, '_, Message>) {
        let backend = self.composer.backend_mut();
        match backend.synchronize(self.application, cx) {
            Ok(()) if backend.should_terminate() => cx.request_termination(),
            Ok(()) => {}
            Err(error) => {
                *self.error.borrow_mut() = Some(error);
                cx.request_termination();
            }
        }
    }

    fn dispatch(&mut self, route: EventRouteId, event: Event) {
        self.composer.dispatch_event(route, event);
    }
}

impl MessageLoopHandler<Message> for AppHandler<'_> {
    fn init(&mut self, cx: &MessageContext<'_, '_, Message>) {
        let sender = cx.sender();
        self.composer
            .backend_mut()
            .set_main_task_sender(move |task| {
                messageloop_appkit::Sender::send(&sender, Message::PerformMain(task)).is_ok()
            });
        self.synchronize(cx);
    }

    fn on(&mut self, cx: &MessageContext<'_, '_, Message>, message: Message) {
        match message {
            Message::PerformMain(task) => task(),
            Message::Command(id) => {
                if let Some(route) = self.composer.backend().main_menu_route() {
                    self.dispatch(route, Event::MenuItemSelected { id });
                }
                self.synchronize(cx);
            }
            Message::Window(event) => {
                let action = WindowEventAction::of(&event.kind);
                if action == WindowEventAction::Closed {
                    self.composer.backend_mut().mark_window_closed(event.window);
                }
                if let Some(route) = event.route {
                    self.dispatch(EventRouteId::from_raw(route.get()), event.kind.shared());
                }
                if action != WindowEventAction::WaitForClose {
                    self.synchronize(cx);
                }
            }
        }
    }
}

/// Runs the app until its last window closes or the user quits.
///
/// `on_exit` runs once before the process ends: after the loop returns, or
/// when AppKit terminates the process directly, as it does for ⌘Q.
pub fn run_composer(
    composer: Composer<RenderNode, AppKitBackend>,
    on_exit: Box<dyn FnOnce()>,
) -> Result<(), AppError> {
    let application = Application::new().map_err(AppError::Application)?;
    let on_exit = Rc::new(RefCell::new(Some(on_exit)));
    let exit = {
        let on_exit = on_exit.clone();
        move || {
            if let Some(action) = on_exit.borrow_mut().take() {
                action();
            }
        }
    };
    application
        .on_will_terminate(exit.clone())
        .map_err(AppError::Ui)?;
    let error = Rc::new(RefCell::new(None));
    let message_loop = MessageLoopAppkit::new(
        &application,
        AppHandler {
            application: &application,
            composer,
            error: error.clone(),
        },
    )
    .map_err(AppError::MessageLoop)?;
    let result = message_loop.run().map_err(AppError::MessageLoop);
    exit();
    result?;
    error.borrow_mut().take().map_or(Ok(()), Err)
}
