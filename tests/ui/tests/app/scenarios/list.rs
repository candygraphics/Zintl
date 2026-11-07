use zintl_ui_desktop::*;

/// A sectioned list bound to a selection Store, with buttons that select and clear from the app.
pub struct MainView {
    selection: Store<Option<String>>,
    picks: Store<Vec<String>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selection: Store::default(),
            picks: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.selection = cx.store(None);
        self.picks = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (selection, picks) = (self.selection, self.picks);
        let row = |id: &str, title: &str| ListRow::new(id, Text::new(title).id(format!("{id}-title")));
        Window::new(Rect::new(100.0, 100.0, 420.0, 460.0), "Zintl List UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    List::new([
                        ListSection::new([row("alpha", "Alpha")]).title("Pinned"),
                        ListSection::new([
                            row("bravo", "Bravo"),
                            row("charlie", "Charlie"),
                            row("delta", "Delta"),
                        ])
                        .title("Recent"),
                    ])
                    .bind(selection)
                    .id("list")
                    .on_select(move |cx, id| {
                        let id = id.unwrap_or("none").to_owned();
                        cx.update(picks, |picks| picks.push(id));
                    }),
                    HStack::new(list![
                        Button::new("Select Charlie").id("select-charlie").on_click(move |cx| {
                            cx.update(selection, |selection| *selection = Some("charlie".into()));
                        }),
                        Button::new("Clear").id("clear").on_click(move |cx| {
                            cx.update(selection, |selection| *selection = None);
                        }),
                    ])
                    .spacing(8.0),
                    cx.watch(selection, |selection| {
                        Text::new(format!("Selected: {}", selection.as_deref().unwrap_or("none")))
                            .id("selection-value")
                    }),
                    cx.watch(picks, |picks| {
                        Text::new(format!("Picks: {}", picks.join(","))).id("picks")
                    }),
                ])
                .spacing(10.0)
                .fill_width(),
            )
    }
}
