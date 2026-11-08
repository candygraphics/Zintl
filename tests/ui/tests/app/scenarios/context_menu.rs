use zintl_ui_desktop::*;

/// A label and a container, each with its own context menu.
pub struct MainView {
    log: Store<Vec<String>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            log: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.log = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let log = self.log;
        let logged = move |title: &'static str| {
            MenuItem::new(title).on_select(move |cx| cx.update(log, |log| log.push(title.into())))
        };
        Window::new(Rect::new(100.0, 100.0, 420.0, 300.0), "Zintl Context Menu UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    Text::new("Station")
                        .id("station")
                        .minimum_size(Size::new(200.0, 24.0))
                        .context_menu([
                            logged("Rename"),
                            logged("Duplicate").shortcut(Shortcut::command("d")),
                            MenuItem::separator(),
                            MenuItem::new("Unavailable"),
                            logged("Remove"),
                        ]),
                    VStack::new(list![Text::new("Line panel").id("line-panel")])
                        .padding(16.0)
                        .context_menu([logged("Edit Line")]),
                    cx.watch(log, |log| Text::new(format!("Log: {}", log.join(","))).id("log")),
                ])
                .spacing(12.0),
            )
    }
}
