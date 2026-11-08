use zintl_ui_desktop::*;

/// A button that shows a popover holding a checkbox and a button that closes it from the app.
pub struct MainView {
    shown: Store<bool>,
    pinned: Store<bool>,
    closes: Store<u32>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            shown: Store::default(),
            pinned: Store::default(),
            closes: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.shown = cx.store(false);
        self.pinned = cx.store(false);
        self.closes = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (shown, pinned, closes) = (self.shown, self.pinned, self.closes);
        let content = VStack::new(list![
            Text::new("Route details").id("popover-text"),
            Toggle::new("Pin route")
                .style(ToggleStyle::Checkbox)
                .bind(pinned)
                .id("pin"),
            Button::new("Done").id("done").on_click(move |cx| {
                cx.update(shown, |shown| *shown = false);
            }),
        ])
        .spacing(8.0);
        Window::new(Rect::new(100.0, 100.0, 420.0, 300.0), "Zintl Popover UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    Button::new("Info")
                        .id("info")
                        .on_click(move |cx| cx.update(shown, |shown| *shown = true))
                        .popover(
                            Popover::new(content)
                                .bind(shown)
                                .edge(PopoverEdge::Bottom)
                                .on_close(move |cx| cx.update(closes, |closes| *closes += 1)),
                        ),
                    cx.watch(shown, |shown| {
                        Text::new(format!("Shown: {shown}")).id("shown-value")
                    }),
                    cx.watch(pinned, |pinned| {
                        Text::new(format!("Pinned: {pinned}")).id("pinned-value")
                    }),
                    cx.watch(closes, |closes| Text::new(format!("Closes: {closes}")).id("closes")),
                ])
                .spacing(10.0),
            )
    }
}
