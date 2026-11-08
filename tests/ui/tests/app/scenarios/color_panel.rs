use zintl_ui_desktop::*;

/// A bound color well and the shared color panel, with buttons that set the color and show the panel.
pub struct MainView {
    color: Store<Color>,
    panel_color: Store<Color>,
    panel_shown: Store<bool>,
    changes: Store<u32>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            color: Store::default(),
            panel_color: Store::default(),
            panel_shown: Store::default(),
            changes: Store::default(),
        }
    }

    /// Writes a color as rounded RGB components.
    fn describe(color: &Color) -> String {
        format!("{:.2} {:.2} {:.2}", color.red, color.green, color.blue)
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.color = cx.store(Color::rgb(1.0, 0.0, 0.0));
        self.panel_color = cx.store(Color::rgb(0.0, 0.0, 0.0));
        self.panel_shown = cx.store(false);
        self.changes = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (color, panel_color, panel_shown, changes) =
            (self.color, self.panel_color, self.panel_shown, self.changes);
        Window::new(Rect::new(100.0, 100.0, 420.0, 300.0), "Zintl Color Panel UI Test")
            .id("main-window")
            .color_panel(ColorPanel::new().bind_color(panel_color).bind(panel_shown))
            .content(
                VStack::new(list![
                    ColorWell::new()
                        .bind(color)
                        .id("well")
                        .on_change(move |cx, _| cx.update(changes, |changes| *changes += 1)),
                    HStack::new(list![
                        Button::new("Blue").id("set-blue").on_click(move |cx| {
                            cx.update(color, |color| *color = Color::rgb(0.0, 0.0, 1.0));
                        }),
                        Button::new("Show Colors").id("show-colors").on_click(move |cx| {
                            cx.update(panel_shown, |shown| *shown = true);
                        }),
                    ])
                    .spacing(8.0),
                    cx.watch(color, |color| {
                        Text::new(format!("Well: {}", Self::describe(color))).id("well-value")
                    }),
                    cx.watch(panel_color, |color| {
                        Text::new(format!("Panel: {}", Self::describe(color))).id("panel-value")
                    }),
                    cx.watch(panel_shown, |shown| {
                        Text::new(format!("Panel shown: {shown}")).id("panel-shown")
                    }),
                    cx.watch(changes, |changes| Text::new(format!("Changes: {changes}")).id("changes")),
                ])
                .spacing(10.0),
            )
    }
}
