use zintl_ui_desktop::*;

/// Three bars and a button that doubles the second one, a chart with short Japanese labels,
/// and two charts that share a label.
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
        let whole = |value: f64| format!("{value:.0}");
        let chart = BarChart::new([
            ChartBar::new("Food", 3480.0),
            ChartBar::new("Transport", *cx.get(transport)),
            ChartBar::new("Rent", 2400.0).color(Color::rgb(0.2, 0.7, 0.3)),
        ])
        .value_format(whole)
        .id("breakdown");
        // Columns sized from these labels must fit two and five CJK characters.
        let japanese = BarChart::new([
            ChartBar::new("水道光熱費", 3480.0).id("cjk-utilities"),
            ChartBar::new("給与", 1200.0).id("cjk-salary"),
        ])
        .value_format(whole)
        .id("cjk");
        // Both charts have an "Other" bar, told apart by ID.
        let expenses = BarChart::new([
            ChartBar::new("Rent", 2000.0).id("expenses-rent"),
            ChartBar::new("Food", 1000.0).id("expenses-food"),
            ChartBar::new("Other", 300.0).id("expenses-other"),
        ])
        .value_format(whole)
        .id("expenses");
        let income = BarChart::new([
            ChartBar::new("Salary", 5000.0).id("income-salary"),
            ChartBar::new("Refunds", 1200.0).id("income-refunds"),
            ChartBar::new("Other", 600.0).id("income-other"),
        ])
        .value_format(whole)
        .id("income");
        Window::new(Rect::new(100.0, 100.0, 480.0, 520.0), "Zintl Chart UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    chart,
                    Button::new("Double transport")
                        .id("double-transport")
                        .on_click(move |cx| cx.update(transport, |value| *value *= 2.0)),
                    japanese,
                    expenses,
                    income,
                ])
                .spacing(12.0)
                .padding(12.0)
                .fill_width(),
            )
    }
}
