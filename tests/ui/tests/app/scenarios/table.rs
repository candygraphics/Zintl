use zintl_ui_desktop::*;

/// A striped table of sections whose selection is shown below and can jump to a far row.
pub struct MainView {
    selected: Store<Option<String>>,
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
        let rows = (1..=50).map(|row| {
            TableRow::new(
                format!("section-{row}"),
                list![
                    Text::new(format!("Section {row}")).id(format!("name-{row}")),
                    Text::new(format!("{row}k000")).id(format!("length-{row}")),
                ],
            )
        });
        let status = cx.watch(selected, |selected| {
            let selected = selected.as_deref().unwrap_or("none");
            Text::new(format!("Selected: {selected}")).id("selection")
        });
        let select = Button::new("Select 40")
            .id("select-40")
            .on_click(move |cx| {
                cx.update(selected, |selected| *selected = Some("section-40".into()))
            });
        let columns = [
            TableColumn::new("Name").width(200.0),
            TableColumn::new("Length"),
        ];
        Window::new(Rect::new(100.0, 100.0, 480.0, 420.0), "Zintl Table UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    status,
                    select,
                    Table::new(columns, rows).bind(selected).id("sections"),
                ])
                .spacing(8.0)
                .fill_width()
                .grow(),
            )
    }
}
