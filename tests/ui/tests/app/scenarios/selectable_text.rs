use zintl_ui_desktop::*;

/// Selectable and plain text, and a table whose amount cells are selectable.
pub struct MainView {
    selected: Store<Option<u32>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selected: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.selected = cx.store(None);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let selected = self.selected;
        let rows = (1..=3).map(|row| {
            TableRow::new(
                row,
                list![
                    Text::new(format!("Entry {row}")).id(format!("name-{row}")),
                    Text::new(format!("{}", row * 10))
                        .selectable(true)
                        .id(format!("amount-{row}")),
                ],
            )
        });
        let columns = [
            TableColumn::new("Entry").width(140.0),
            TableColumn::new("Amount")
                .width(100.0)
                .alignment(TextAlignment::Trailing),
        ];
        Window::new("Zintl Selectable Text UI Test")
            .size(Size::new(360.0, 300.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    Text::new("1,234")
                        .font(Font::system(15.0))
                        .selectable(true)
                        .id("total"),
                    Text::new("5,678").id("plain"),
                    cx.watch(selected, |selected| {
                        let selected = selected.map_or("none".to_owned(), |row| row.to_string());
                        Text::new(format!("Selected: {selected}")).id("selection")
                    }),
                    Table::new(columns, rows).bind(selected).id("entries"),
                ])
                .spacing(8.0)
                .fill_width(),
            )
    }
}
