use zintl_ui_desktop::*;

/// A striped table of sections whose selection is shown below and can jump to a far row.
pub struct MainView {
    selected: Store<Option<String>>,
    activated: Store<String>,
    deleted: Store<String>,
    note: Store<String>,
    log: Store<String>,
    locked: Store<bool>,
    sort: Store<Option<SortOrder>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selected: Store::default(),
            activated: Store::default(),
            deleted: Store::default(),
            note: Store::default(),
            log: Store::default(),
            locked: Store::default(),
            sort: Store::default(),
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
        self.log = cx.store(String::new());
        self.locked = cx.store(false);
        self.sort = cx.store(None);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (selected, activated, deleted, log, locked, sort) = (
            self.selected,
            self.activated,
            self.deleted,
            self.log,
            self.locked,
            self.sort,
        );
        // The app sorts its own rows by the order the header reports.
        let descending = cx.get(sort).as_ref().is_some_and(|order| !order.ascending);
        let order: Vec<u32> = if descending {
            (1..=50).rev().collect()
        } else {
            (1..=50).collect()
        };
        let rows = order.into_iter().map(|row| {
            let table_row = TableRow::new(
                format!("section-{row}"),
                list![
                    Text::new(format!("Section {row}")).id(format!("name-{row}")),
                    Text::new(format!("{row}k000")).id(format!("length-{row}")),
                ],
            );
            // Every row but the seventh has a menu.
            if row == 7 {
                return table_row;
            }
            let title = format!("Archive Section {row}");
            table_row.context_menu([MenuItem::new(title.clone())
                .on_select(move |cx| cx.update(log, |log| log.clone_from(&title)))])
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
        let sort_descending = Button::new("Sort descending")
            .id("sort-descending")
            .on_click(move |cx| {
                cx.update(sort, |sort| {
                    *sort = Some(SortOrder {
                        key: "name".into(),
                        ascending: false,
                    })
                })
            });
        // Buttons do not take focus, so the table keeps it after the selection is cleared.
        let clear = Button::new("Clear selection")
            .id("clear-selection")
            .on_click(move |cx| cx.update(selected, |selected| *selected = None));
        let columns = [
            TableColumn::new("Name").width(200.0).sort_key("name"),
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
                    HStack::new(list![
                        select,
                        clear,
                        sort_descending,
                        // While on, the table keeps its selection.
                        Toggle::new("Lock")
                            .style(ToggleStyle::Checkbox)
                            .bind(locked)
                            .id("lock-selection"),
                    ])
                    .spacing(8.0),
                    cx.watch(activated, |id| Text::new(format!("Activated: {id}")).id("activated")),
                    cx.watch(deleted, |id| Text::new(format!("Deleted: {id}")).id("deleted")),
                    cx.watch(log, |log| Text::new(format!("Log: {log}")).id("log")),
                    cx.watch(sort, |sort| {
                        let sort = sort.as_ref().map_or("none".to_owned(), |order| {
                            let direction = if order.ascending { "ascending" } else { "descending" };
                            format!("{} {direction}", order.key)
                        });
                        Text::new(format!("Sort: {sort}")).id("sort")
                    }),
                    // Delete typed here edits the text instead of reaching the table.
                    TextField::new().id("note").bind(self.note),
                    Table::new(columns, rows)
                        .bind(selected)
                        .on_activate(move |cx, id| cx.update(activated, |activated| *activated = id.into()))
                        .on_delete(move |cx, id| cx.update(deleted, |deleted| *deleted = id.into()))
                        .should_select(move |cx, _| !*cx.get(locked))
                        .bind_sort(sort)
                        .id("sections"),
                ])
                .spacing(8.0)
                .fill_width(),
            )
    }
}
