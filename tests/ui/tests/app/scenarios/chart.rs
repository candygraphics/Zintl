use zintl_ui_desktop::*;

/// Three bars and a button that doubles the second one.
pub struct MainView {
    transport: Store<f64>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            transport: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.transport = cx.store(1200.0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let transport = self.transport;
        let chart = BarChart::new([
            ChartBar::new("Food", 3480.0),
            ChartBar::new("Transport", *cx.get(transport)),
            ChartBar::new("Rent", 2400.0).color(Color::rgb(0.2, 0.7, 0.3)),
        ])
        .value_format(|value| format!("{value:.0}"))
        .id("breakdown");
        Window::new(Rect::new(100.0, 100.0, 480.0, 240.0), "Zintl Chart UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    chart,
                    Button::new("Double transport")
                        .id("double-transport")
                        .on_click(move |cx| cx.update(transport, |value| *value *= 2.0)),
                ])
                .spacing(12.0)
                .padding(12.0)
                .fill_width(),
            )
    }
}
