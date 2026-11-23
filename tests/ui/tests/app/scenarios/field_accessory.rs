use zintl_ui_desktop::dialog::{Popover, PopoverEdge};
use zintl_ui_desktop::*;

/// Fields with a calculator button inside them: a text field whose button opens a popover
/// anchored to the field, an integer field, and a grid comparing widths with and without one.
pub struct MainView {
    amount: Store<String>,
    count: Store<Option<i64>>,
    clicks: Store<u32>,
    shown: Store<bool>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            amount: Store::default(),
            count: Store::default(),
            clicks: Store::default(),
            shown: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.amount = cx.store(String::new());
        self.count = cx.store(Some(5));
        self.clicks = cx.store(0);
        self.shown = cx.store(false);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (amount, count, clicks, shown) = (self.amount, self.count, self.clicks, self.shown);
        let calculator = |id: &str| {
            FieldAccessory::button("plus.forwardslash.minus", "Calculator")
                .id(id.to_owned())
                .on_click(move |cx| cx.update(clicks, |clicks| *clicks += 1))
        };
        Window::new("Zintl Field Accessory UI Test")
            .size(Size::new(460.0, 360.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    TextField::new()
                        .bind(amount)
                        .id("amount")
                        .minimum_size(Size::new(240.0, 0.0))
                        .accessory(
                            FieldAccessory::button("plus.forwardslash.minus", "Calculator")
                                .id("amount-calculator")
                                .on_click(move |cx| {
                                    cx.update(clicks, |clicks| *clicks += 1);
                                    cx.update(shown, |shown| *shown = true);
                                }),
                        )
                        .popover(
                            Popover::new(Text::new("Calculator").id("calculator"))
                                .bind_presented(shown)
                                .edge(PopoverEdge::Bottom),
                        ),
                    IntegerField::optional()
                        .bind(count)
                        .id("count")
                        .minimum_size(Size::new(160.0, 0.0))
                        .accessory(calculator("count-calculator")),
                    // Both fields fill the same column, with and without a button.
                    Grid::new([
                        GridRow::new(list![
                            Text::new("Plain"),
                            TextField::new().id("plain-field"),
                        ]),
                        GridRow::new(list![
                            Text::new("With button"),
                            TextField::new()
                                .id("accessory-field")
                                .accessory(calculator("grid-calculator")),
                        ]),
                    ])
                    .spacing(6.0),
                    cx.watch(amount, |amount| Text::new(format!("Amount: {amount}")).id("amount-value")),
                    cx.watch(count, |count| {
                        let count = count.map_or("None".to_owned(), |count| count.to_string());
                        Text::new(format!("Count: {count}")).id("count-value")
                    }),
                    cx.watch(clicks, |clicks| Text::new(format!("Clicks: {clicks}")).id("clicks")),
                ])
                .spacing(10.0),
            )
    }
}
