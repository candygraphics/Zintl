use zintl_ui_desktop::*;

/// Buttons with each role and several bezel styles; every click is logged by name.
pub struct MainView {
    log: Store<Vec<&'static str>>,
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
        let logged = move |name: &'static str| {
            move |cx: &mut Context<'_>| cx.update(log, |log| log.push(name))
        };
        Window::new("Zintl Buttons UI Test").size(Size::new(520.0, 340.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    HStack::new(list![
                        Button::new("Save")
                            .role(ButtonRole::Default)
                            .id("save")
                            .on_click(logged("save")),
                        Button::new("Cancel")
                            .role(ButtonRole::Cancel)
                            .id("cancel")
                            .on_click(logged("cancel")),
                        Button::new("Delete")
                            .role(ButtonRole::Destructive)
                            .id("delete")
                            .on_click(logged("delete")),
                        Button::new("Plain").id("plain").on_click(logged("plain")),
                    ])
                    .spacing(8.0),
                    HStack::new(list![
                        Button::icon("gearshape")
                            .style(ButtonStyle::Borderless)
                            .id("gear")
                            .on_click(logged("gear")),
                        Button::icon("plus")
                            .style(ButtonStyle::Circular)
                            .id("circular")
                            .on_click(logged("circular")),
                        Button::help()
                            .id("help")
                            .on_click(logged("help")),
                        Button::new("Unavailable")
                            .enabled(false)
                            .id("disabled")
                            .on_click(logged("disabled")),
                    ])
                    .spacing(8.0),
                    // Icon-only push buttons take their symbol's size.
                    HStack::new(list![
                        Button::icon("chevron.left")
                            .id("previous")
                            .on_click(logged("previous")),
                        Button::icon("chevron.right")
                            .id("next")
                            .on_click(logged("next")),
                    ])
                    .spacing(8.0),
                    // Typing here checks that Return reaches the default button from a text field.
                    TextField::new().id("note").minimum_size(Size::new(200.0, 24.0)),
                    cx.watch(log, |log| Text::new(format!("Log: {}", log.join(","))).id("log")),
                ])
                .spacing(12.0),
            )
    }
}
