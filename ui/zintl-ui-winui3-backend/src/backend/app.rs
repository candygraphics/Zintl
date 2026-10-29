use std::cell::RefCell;
use std::rc::Rc;

use messageloop_dispatcherqueue::{
    Context as MessageContext, DispatcherQueueSender, MessageLoopDispatcherQueue,
    MessageLoopHandler,
};
use zintl_ui::composer::Composer;
use zintl_ui::event::EventRouteId;
use zintl_ui_backend_core::{Event, MainTask, NodeId, RenderNode};
use zpd_winui3::Application;

use super::menu::Menus;
use super::{AppError, WinUI3Backend};

pub(super) enum Message {
    /// A control or presentation reported a user change.
    Event {
        route: EventRouteId,
        event: Event,
    },
    /// The user or the app closed a window.
    WindowClosed {
        route: Option<EventRouteId>,
    },
    PerformMain(MainTask),
    /// A menu bar item was chosen in `window`.
    Command {
        id: String,
        window: NodeId,
    },
}

pub(super) type MessageSender = DispatcherQueueSender<Message>;

struct AppHandler {
    composer: Composer<RenderNode, WinUI3Backend>,
    error: Rc<RefCell<Option<AppError>>>,
}

impl AppHandler {
    fn synchronize(&mut self, cx: &MessageContext<'_, Message>) {
        let backend = self.composer.backend_mut();
        match backend.synchronize(cx.application(), &cx.sender()) {
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

impl MessageLoopHandler<Message> for AppHandler {
    fn init(&mut self, cx: &MessageContext<'_, Message>) {
        let sender = cx.sender();
        self.composer.backend_mut().connect_main_tasks(move |task| {
            messageloop_dispatcherqueue::Sender::send(&sender, Message::PerformMain(task)).is_ok()
        });
        self.synchronize(cx);
    }

    fn on(&mut self, cx: &MessageContext<'_, Message>, message: Message) {
        match message {
            Message::PerformMain(task) => task(),
            Message::Event { route, event } => {
                self.dispatch(route, event);
                self.synchronize(cx);
            }
            Message::WindowClosed { route } => {
                if let Some(route) = route {
                    self.dispatch(route, Event::WindowWillClose);
                    self.dispatch(route, Event::WindowDidClose);
                }
                self.synchronize(cx);
            }
            Message::Command { id, window } => match id.as_str() {
                Menus::QUIT => cx.request_termination(),
                Menus::CLOSE => self.composer.backend().close_window(window),
                _ => {
                    if let Some(route) = self.composer.backend().main_menu_route() {
                        self.dispatch(route, Event::MenuItemSelected { id });
                    }
                    self.synchronize(cx);
                }
            },
        }
    }
}

/// Runs `composer` in a WinUI application until its last normal window closes.
pub fn run_composer(composer: Composer<RenderNode, WinUI3Backend>) -> Result<(), AppError> {
    let application = Application::new().map_err(AppError::Application)?;
    let error = Rc::new(RefCell::new(None));
    MessageLoopDispatcherQueue::new(
        application,
        AppHandler {
            composer,
            error: error.clone(),
        },
    )
    .run()
    .map_err(AppError::MessageLoop)?;
    error.borrow_mut().take().map_or(Ok(()), Err)
}
