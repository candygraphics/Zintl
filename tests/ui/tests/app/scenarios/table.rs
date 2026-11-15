use zintl_ui_desktop::*;

/// A striped table of sections whose selection is shown below and can jump to a far row.
pub struct MainView {
    selected: Store<Option<String>>,
    activated: Store<String>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selected: Store::default(),
            activated: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.selected = cx.store(None);
        self.activated = cx.store(String::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (selected, activated) = (self.selected, self.activated);
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
            // Cells keep the default alignment, so they follow the column's.
            TableColumn::new("Length")
                .width(120.0)
                .alignment(TextAlignment::Trailing),
        ];
        Window::new(Rect::new(100.0, 100.0, 480.0, 420.0), "Zintl Table UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    status,
                    select,
                    cx.watch(activated, |id| Text::new(format!("Activated: {id}")).id("activated")),
                    Table::new(columns, rows)
                        .bind(selected)
                        .on_activate(move |cx, id| cx.update(activated, |activated| *activated = id.into()))
                        .id("sections"),
                ])
                .spacing(8.0)
                .fill_width(),
            )
    }
}
