use zintl_ui_desktop::*;

/// An integer field with thousands separators, a button that sets a value above 2^53,
/// a button that copies the Store as a click would read it, and an optional field.
pub struct MainView {
    quantity: Store<i64>,
    copied: Store<Option<i64>>,
    count: Store<Option<i64>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            quantity: Store::default(),
            copied: Store::default(),
            count: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.quantity = cx.store(3);
        self.copied = cx.store(None);
        self.count = cx.store(Some(7));
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (quantity, copied, count) = (self.quantity, self.copied, self.count);
        Window::new(Rect::new(100.0, 100.0, 420.0, 380.0), "Zintl Integer Field UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    NumberField::integer()
                        .range(0..=i64::MAX)
                        .grouping(true)
                        .bind(quantity)
                        .id("quantity")
                        .minimum_size(Size::new(220.0, 22.0)),
                    // 2^53 + 1 cannot be represented as an f64.
                    Button::new("Set 2^53 + 1").id("set-big").on_click(move |cx| {
                        cx.update(quantity, |quantity| *quantity = 9_007_199_254_740_993)
                    }),
                    // Push buttons do not end editing, so this reads what typing reported.
                    Button::new("Copy").id("copy").on_click(move |cx| {
                        let value = *cx.get(quantity);
                        cx.update(copied, |copied| *copied = Some(value));
                    }),
                    cx.watch(quantity, |quantity| {
                        Text::new(format!("Integer: {quantity}")).id("integer-value")
                    }),
                    cx.watch(copied, |copied| {
                        let copied = copied.map_or("-".to_owned(), |value| value.to_string());
                        Text::new(format!("Copied: {copied}")).id("copied-value")
                    }),
                    NumberField::integer_optional()
                        .range(0..=100)
                        .placeholder("None")
                        .bind(count)
                        .id("count")
                        .minimum_size(Size::new(220.0, 22.0)),
                    cx.watch(count, |count| {
                        let count = count.map_or("None".to_owned(), |value| value.to_string());
                        Text::new(format!("Count: {count}")).id("count-value")
                    }),
                ])
                .spacing(10.0),
            )
    }
}
