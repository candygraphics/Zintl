use zintl_ui_desktop::*;

/// One toggle per style, a disabled toggle, and buttons that set them all from the app.
pub struct MainView {
    switch: Store<bool>,
    checkbox: Store<bool>,
    button: Store<bool>,
    disabled: Store<bool>,
    changes: Store<Vec<bool>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            switch: Store::default(),
            checkbox: Store::default(),
            button: Store::default(),
            disabled: Store::default(),
            changes: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.switch = cx.store(false);
        self.checkbox = cx.store(false);
        self.button = cx.store(false);
        self.disabled = cx.store(true);
        self.changes = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (switch, checkbox, button, changes) =
            (self.switch, self.checkbox, self.button, self.changes);
        let set_all = move |on: bool| {
            move |cx: &mut Context<'_>| {
                for store in [switch, checkbox, button] {
                    cx.update(store, |value| *value = on);
                }
            }
        };
        let echo = |cx: &mut Context<'_>, store: Store<bool>, id: &'static str| {
            cx.watch(store, move |on| {
                Text::new(if *on { "On" } else { "Off" }).id(id)
            })
        };
        Window::new(Rect::new(100.0, 100.0, 420.0, 360.0), "Zintl Toggles UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    Toggle::new("Wi-Fi")
                        .bind(switch)
                        .id("switch")
                        .on_change(move |cx, on| cx.update(changes, |changes| changes.push(on))),
                    Toggle::new("Show hidden files")
                        .style(ToggleStyle::Checkbox)
                        .bind(checkbox)
                        .id("checkbox"),
                    Toggle::new("Bold")
                        .style(ToggleStyle::Button)
                        .bind(button)
                        .id("button-toggle"),
                    Toggle::new("Locked")
                        .style(ToggleStyle::Checkbox)
                        .bind(self.disabled)
                        .enabled(false)
                        .id("disabled-checkbox"),
                    HStack::new(list![
                        Button::new("All on").id("all-on").on_click(set_all(true)),
                        Button::new("All off").id("all-off").on_click(set_all(false)),
                    ])
                    .spacing(8.0),
                    echo(cx, switch, "switch-value"),
                    echo(cx, checkbox, "checkbox-value"),
                    echo(cx, button, "button-toggle-value"),
                    echo(cx, self.disabled, "disabled-value"),
                    cx.watch(changes, |changes| {
                        let changes = changes
                            .iter()
                            .map(|on| if *on { "on" } else { "off" })
                            .collect::<Vec<_>>();
                        Text::new(format!("Changes: {}", changes.join(","))).id("changes")
                    }),
                ])
                .spacing(10.0),
            )
    }
}
