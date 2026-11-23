use zintl_ui_desktop::*;

/// A budget table with more rows than fit, sorted by a Store, and a totals row below the rows.
pub struct MainView {
    selected: Store<Option<u32>>,
    selections: Store<u32>,
    descending: Store<bool>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selected: Store::default(),
            selections: Store::default(),
            descending: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.selected = cx.store(None);
        self.selections = cx.store(0);
        self.descending = cx.store(false);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (selected, selections, descending) = (self.selected, self.selections, self.descending);
        let mut order: Vec<u32> = (1..=30).collect();
        if *cx.get(descending) {
            order.reverse();
        }
        let rows = order.into_iter().map(|row| {
            TableRow::new(
                row,
                list![
                    Text::new(format!("Category {row}")).id(format!("name-{row}")),
                    Text::new(format!("{}", row * 10)).id(format!("budget-{row}")),
                    Text::new(format!("{}", row * 7)).id(format!("spent-{row}")),
                ],
            )
        });
        let columns = [
            TableColumn::new("Category").width(160.0),
            TableColumn::new("Budget")
                .width(100.0)
                .alignment(TextAlignment::Trailing),
            TableColumn::new("Spent")
                .width(100.0)
                .alignment(TextAlignment::Trailing),
        ];
        Window::new("Zintl Table Summary UI Test")
            .size(Size::new(420.0, 360.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    cx.watch(selected, |selected| {
                        let selected = selected.map_or("none".to_owned(), |row| row.to_string());
                        Text::new(format!("Selected: {selected}")).id("selection")
                    }),
                    cx.watch(selections, |count| {
                        Text::new(format!("Selections: {count}")).id("selections")
                    }),
                    Toggle::new("Descending")
                        .style(ToggleStyle::Checkbox)
                        .bind(descending)
                        .id("descending"),
                    Table::new(columns, rows)
                        .bind(selected)
                        .on_select(move |cx, _| cx.update(selections, |count| *count += 1))
                        .summary_row(list![
                            Text::new("Total").id("total-name"),
                            Text::new("4650").id("total-budget"),
                            Text::new("3255").id("total-spent"),
                        ])
                        .id("budget"),
                ])
                .spacing(8.0)
                .fill_width(),
            )
    }
}
