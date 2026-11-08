use zintl_ui_desktop::*;

/// A bound horizontal split limited to 120...300 points, and a vertical split that starts clamped.
pub struct MainView {
    width: Store<f32>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            width: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.width = cx.store(200.0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let width = self.width;
        // The vertical split asks for 20 points but its range starts at 60.
        let detail = SplitView::vertical(
            Text::new("Top").id("top-pane").fill_width(),
            Text::new("Bottom").id("bottom-pane").fill_width(),
        )
        .position(20.0)
        .range(60.0..=200.0);
        Window::new(Rect::new(100.0, 100.0, 640.0, 420.0), "Zintl Split View UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    HStack::new(list![
                        Button::new("Reset width").id("reset-width").on_click(move |cx| {
                            cx.update(width, |width| *width = 150.0);
                        }),
                        cx.watch(width, |width| Text::new(format!("Width: {width:.0}")).id("width-value")),
                    ])
                    .spacing(8.0),
                    SplitView::horizontal(Text::new("Sidebar").id("first-pane"), detail)
                        .range(120.0..=300.0)
                        .bind(width),
                ])
                .spacing(8.0)
                .fill_width()
                .grow(),
            )
    }
}
