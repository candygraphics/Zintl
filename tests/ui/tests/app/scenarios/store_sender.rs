use std::thread;
use std::time::Duration;

use zintl_ui_desktop::*;

/// Builds on a worker thread, which reports back through a StoreSender.
pub struct MainView {
    status: Store<String>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            status: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.status = cx.store("Idle".to_owned());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let status = self.status;
        Window::new(
            Rect::new(100.0, 100.0, 480.0, 240.0),
            "Zintl Store Sender UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                Button::new("Build").id("build").on_click(move |cx| {
                    cx.update(status, |status| *status = "Building".to_owned());
                    let sender = cx.sender(status);
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(300));
                        sender.set("Built".to_owned());
                    });
                }),
                cx.watch(status, |status| Text::new(status.clone())
                    .id("build-status")),
            ])
            .spacing(12.0),
        )
    }
}
