use std::cell::RefCell;
use std::rc::Rc;

use messageloop_appkit::{Context as MessageContext, MessageLoopAppkit, MessageLoopHandler};
use zintl_ui::composer::Composer;
use zintl_ui::event::EventRouteId;
use zpd_appkit::actor::{WindowEvent, WindowEventKind};
use zpd_appkit::runloop::Application;

use super::{AppError, AppKitBackend};
use crate::convert::Shared;
use crate::{AppKitEvent, AppKitRenderNode, MainTask};

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

struct AppHandler<'application, R>
where
    R: AppKitRenderNode,
{
    application: &'application Application,
    composer: Composer<R, AppKitBackend<R>>,
    error: Rc<RefCell<Option<AppError>>>,
}

impl<R> AppHandler<'_, R>
where
    R: AppKitRenderNode,
{
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

    fn dispatch(&mut self, route: EventRouteId, event: AppKitEvent) {
        self.composer.dispatch_event(route, R::appkit_event(event));
    }
}

impl<R> MessageLoopHandler<Message> for AppHandler<'_, R>
where
    R: AppKitRenderNode,
{
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
                    self.dispatch(route, AppKitEvent::MenuItemSelected { id });
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

pub fn run_composer<R>(composer: Composer<R, AppKitBackend<R>>) -> Result<(), AppError>
where
    R: AppKitRenderNode,
{
    let application = Application::new().map_err(AppError::Application)?;
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
    message_loop.run().map_err(AppError::MessageLoop)?;
    error.borrow_mut().take().map_or(Ok(()), Err)
}
