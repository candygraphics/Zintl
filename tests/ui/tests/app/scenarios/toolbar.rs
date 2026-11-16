use zintl_ui_desktop::*;

/// A toolbar whose Remove button is enabled only while there are items, plus a search field
/// that a button and ⌘F focus.
pub struct MainView {
    count: Store<u32>,
    removes: Store<u32>,
    query: Store<String>,
    search_focus: Trigger,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            count: Store::default(),
            removes: Store::default(),
            query: Store::default(),
            search_focus: Trigger::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.count = cx.store(0);
        self.removes = cx.store(0);
        self.query = cx.store(String::new());
        self.search_focus = cx.trigger();
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (count, removes, query) = (self.count, self.removes, self.query);
        let search_focus = self.search_focus;
        Window::new(Rect::new(100.0, 100.0, 560.0, 260.0), "Zintl Toolbar UI Test")
            .id("main-window")
            .toolbar(Toolbar::new([
                ToolbarItem::button("add", "Add", "plus")
                    .on_click(move |cx| cx.update(count, |count| *count += 1)),
                ToolbarItem::button("remove", "Remove", "minus")
                    .enabled(*cx.get(count) > 0)
                    .on_click(move |cx| {
                        cx.update(count, |count| *count = count.saturating_sub(1));
                        cx.update(removes, |removes| *removes += 1);
                    }),
                ToolbarItem::flexible_space(),
                ToolbarItem::search(query)
                    .placeholder("Search")
                    .focus_on(search_focus)
                    .id("toolbar-search"),
            ]))
            .content(
                VStack::new(list![
                    cx.watch(count, |count| Text::new(format!("Count: {count}")).id("count")),
                    cx.watch(removes, |removes| Text::new(format!("Removes: {removes}"))
                        .id("removes")),
                    cx.watch(query, |query| Text::new(format!("Query: {query}")).id("query-value")),
                    Button::new("Set Query")
                        .id("set-query")
                        .on_click(move |cx| cx.update(query, |query| *query = "Osaka".into())),
                    Button::new("Find")
                        .id("find")
                        .on_click(move |cx| cx.fire(search_focus, ())),
                ])
                .spacing(10.0),
            )
    }
}
