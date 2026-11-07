use zintl_ui_desktop::*;

/// A searchable sidebar whose items carry badges and context menus.
pub struct MainView {
    selection: Store<Option<String>>,
    query: Store<String>,
    log: Store<Vec<String>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selection: Store::default(),
            query: Store::default(),
            log: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.selection = cx.store(Some("inbox".into()));
        self.query = cx.store(String::new());
        self.log = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (selection, query, log) = (self.selection, self.query, self.log);
        let logged = move |entry: &'static str| {
            MenuItem::new(entry.to_owned()).on_select(move |cx| cx.update(log, |log| log.push(entry.into())))
        };
        Window::new(Rect::new(100.0, 100.0, 640.0, 420.0), "Zintl Sidebar UI Test")
            .id("main-window")
            .sidebar(
                Sidebar::new([SidebarSection::new([
                    SidebarItem::new("inbox", "Inbox").system_image("tray").badge("12"),
                    SidebarItem::new("drafts", "Drafts")
                        .system_image("doc")
                        .context_menu([logged("Rename Drafts"), logged("Archive Drafts")]),
                    SidebarItem::new("sent", "Sent").system_image("paperplane"),
                ])
                .title("Mail")])
                .bind(selection)
                .searchable(query)
                .search_placeholder("Filter"),
            )
            .content(
                VStack::new(list![
                    cx.watch(selection, |selection| {
                        Text::new(format!("Selected: {}", selection.as_deref().unwrap_or("none")))
                            .id("selection-value")
                    }),
                    cx.watch(query, |query| Text::new(format!("Query: {query}")).id("query-value")),
                    cx.watch(log, |log| Text::new(format!("Log: {}", log.join(","))).id("log")),
                    Button::new("Show Sent").id("show-sent").on_click(move |cx| {
                        cx.update(selection, |selection| *selection = Some("sent".into()));
                    }),
                ])
                .spacing(10.0),
            )
    }
}
