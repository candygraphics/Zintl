use zintl_ui_desktop::*;

/// A main window, a bound floating inspector panel over it, and windows with each title bar style.
pub struct MainView {
    inspector: Store<bool>,
    closes: Store<u32>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            inspector: Store::default(),
            closes: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.inspector = cx.store(true);
        self.closes = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (inspector, closes) = (self.inspector, self.closes);
        // `extends` lets the content extend under the title bar instead of starting below it.
        let titled = |id: &'static str, x: f64, y: f64, title_bar: TitleBar, extends: bool| {
            let content = VStack::new(list![Text::new("Top").id(format!("{id}-top"))]);
            let content = if extends {
                content.ignores_safe_area()
            } else {
                content
            };
            Window::new(Rect::new(x, y, 300.0, 200.0), format!("Zintl {id}"))
                .id(id)
                .title_bar(title_bar)
                .content(content)
        };
        list![
            Window::new(Rect::new(100.0, 100.0, 480.0, 320.0), "Zintl Windows UI Test")
                .id("main-window")
                .content(
                    VStack::new(list![
                        Text::new("Top").id("main-window-top"),
                        Button::new("Open Inspector").id("open-inspector").on_click(move |cx| {
                            cx.update(inspector, |open| *open = true);
                        }),
                        cx.watch(inspector, |open| {
                            Text::new(format!("Inspector: {open}")).id("inspector-value")
                        }),
                        cx.watch(closes, |closes| {
                            Text::new(format!("Closes: {closes}")).id("closes")
                        }),
                    ])
                    .spacing(10.0),
                ),
            // The panel overlaps the main window's right side.
            Window::panel(Rect::new(420.0, 160.0, 240.0, 180.0), "Inspector")
                .id("inspector")
                .bind_open(inspector)
                .on_close(move |cx| cx.update(closes, |closes| *closes += 1))
                .content(Text::new("Inspector content").id("inspector-content")),
            titled("overlay-window", 700.0, 100.0, TitleBar::Overlay, true),
            titled("hidden-window", 700.0, 360.0, TitleBar::Hidden, true),
            titled("overlay-safe-window", 1020.0, 100.0, TitleBar::Overlay, false),
        ]
    }
}
