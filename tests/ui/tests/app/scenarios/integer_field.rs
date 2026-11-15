use zintl_ui_desktop::*;

/// An integer field with thousands separators and a button that sets a value above 2^53.
pub struct MainView {
    quantity: Store<i64>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            quantity: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.quantity = cx.store(3);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let quantity = self.quantity;
        Window::new(Rect::new(100.0, 100.0, 420.0, 220.0), "Zintl Integer Field UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    NumberField::integer()
                        .range(0..=i64::MAX)
                        .grouping(true)
                        .bind(quantity)
                        .id("quantity")
                        .minimum_size(Size::new(220.0, 0.0)),
                    // 2^53 + 1 cannot be represented as an f64.
                    Button::new("Set 2^53 + 1").id("set-big").on_click(move |cx| {
                        cx.update(quantity, |quantity| *quantity = 9_007_199_254_740_993)
                    }),
                    cx.watch(quantity, |quantity| {
                        Text::new(format!("Integer: {quantity}")).id("integer-value")
                    }),
                ])
                .spacing(10.0),
            )
    }
}
