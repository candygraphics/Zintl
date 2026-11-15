use zintl_ui_desktop::*;

/// A striped table of sections whose selection is shown below and can jump to a far row.
pub struct MainView {
    selected: Store<Option<String>>,
    activated: Store<String>,
    deleted: Store<String>,
    note: Store<String>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selected: Store::default(),
            activated: Store::default(),
            deleted: Store::default(),
            note: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.selected = cx.store(None);
        self.activated = cx.store(String::new());
        self.deleted = cx.store(String::new());
        self.note = cx.store("draft".into());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (selected, activated, deleted) = (self.selected, self.activated, self.deleted);
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
        // Buttons do not take focus, so the table keeps it after the selection is cleared.
        let clear = Button::new("Clear selection")
            .id("clear-selection")
            .on_click(move |cx| cx.update(selected, |selected| *selected = None));
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
                    HStack::new(list![select, clear]).spacing(8.0),
                    cx.watch(activated, |id| Text::new(format!("Activated: {id}")).id("activated")),
                    cx.watch(deleted, |id| Text::new(format!("Deleted: {id}")).id("deleted")),
                    // Delete typed here edits the text instead of reaching the table.
                    TextField::new().id("note").bind(self.note),
                    Table::new(columns, rows)
                        .bind(selected)
                        .on_activate(move |cx, id| cx.update(activated, |activated| *activated = id.into()))
                        .on_delete(move |cx, id| cx.update(deleted, |deleted| *deleted = id.into()))
                        .id("sections"),
                ])
                .spacing(8.0)
                .fill_width(),
            )
    }
}
