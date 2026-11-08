use std::thread;
use std::time::Duration;

use zintl_ui_desktop::*;

/// A confirmation alert with destructive and cancel buttons, an alert that the app dismisses
/// itself, and an alert without buttons.
pub struct MainView {
    confirm: Store<bool>,
    timed: Store<bool>,
    notice: Store<bool>,
    log: Store<Vec<&'static str>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            confirm: Store::default(),
            timed: Store::default(),
            notice: Store::default(),
            log: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.confirm = cx.store(false);
        self.timed = cx.store(false);
        self.notice = cx.store(false);
        self.log = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (confirm, timed, notice, log) = (self.confirm, self.timed, self.notice, self.log);
        let logged = move |entry: &'static str| {
            move |cx: &mut Context<'_>| cx.update(log, |log| log.push(entry))
        };
        let show = |store: Store<bool>| move |cx: &mut Context<'_>| cx.update(store, |shown| *shown = true);
        let echo = |cx: &mut Context<'_>, store: Store<bool>, id: &'static str| {
            cx.watch(store, move |shown| Text::new(format!("{shown}")).id(id))
        };
        Window::new(Rect::new(100.0, 100.0, 480.0, 360.0), "Zintl Alert UI Test")
            .id("main-window")
            .alert(
                Alert::new("Delete the route?")
                    .message("This cannot be undone.")
                    .style(AlertStyle::Critical)
                    .button(
                        AlertButton::new("Delete")
                            .role(ButtonRole::Destructive)
                            .on_click(logged("delete")),
                    )
                    .button(
                        AlertButton::new("Keep")
                            .role(ButtonRole::Cancel)
                            .on_click(logged("keep")),
                    )
                    .bind(confirm),
            )
            .alert(
                Alert::new("Closing soon")
                    .button(AlertButton::new("Wait").on_click(logged("wait")))
                    .bind(timed),
            )
            .alert(Alert::new("Sync finished").bind(notice))
            .content(
                VStack::new(list![
                    Button::new("Delete route").id("delete-route").on_click(show(confirm)),
                    Button::new("Show timed alert").id("show-timed").on_click(move |cx| {
                        cx.update(timed, |shown| *shown = true);
                        // The app dismisses the alert itself while it is shown.
                        let sender = cx.sender(timed);
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(800));
                            sender.set(false);
                        });
                    }),
                    Button::new("Show notice").id("show-notice").on_click(show(notice)),
                    echo(cx, confirm, "confirm-value"),
                    echo(cx, timed, "timed-value"),
                    echo(cx, notice, "notice-value"),
                    cx.watch(log, |log| Text::new(format!("Log: {}", log.join(","))).id("log")),
                ])
                .spacing(10.0),
            )
    }
}
