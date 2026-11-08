use zintl_ui_desktop::*;

/// Three tabs with one labelled page each and a button that opens the last tab.
pub struct MainView {
    selected: Store<usize>,
    verbose: Store<bool>,
    changes: Store<Vec<usize>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selected: Store::default(),
            verbose: Store::default(),
            changes: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.selected = cx.store(0);
        self.verbose = cx.store(false);
        self.changes = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (selected, changes) = (self.selected, self.changes);
        Window::new(Rect::new(100.0, 100.0, 480.0, 360.0), "Zintl Tabs UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    TabView::new([
                        Tab::new("General", Text::new("General settings").id("general-page")),
                        Tab::new(
                            "Advanced",
                            VStack::new(list![
                                Text::new("Advanced settings").id("advanced-page"),
                                Toggle::new("Verbose logging")
                                    .style(ToggleStyle::Checkbox)
                                    .bind(self.verbose)
                                    .id("verbose"),
                            ]),
                        ),
                        Tab::new("About", Text::new("About this app").id("about-page")),
                    ])
                    .bind(selected)
                    .id("tabs")
                    .on_change(move |cx, index| cx.update(changes, |changes| changes.push(index))),
                    Button::new("Show About").id("show-about").on_click(move |cx| {
                        cx.update(selected, |selected| *selected = 2);
                    }),
                    cx.watch(selected, |selected| Text::new(format!("Tab: {selected}")).id("tab-value")),
                    cx.watch(self.verbose, |on| Text::new(format!("Verbose: {on}")).id("verbose-value")),
                    cx.watch(changes, |changes| {
                        let changes = changes.iter().map(usize::to_string).collect::<Vec<_>>();
                        Text::new(format!("Changes: {}", changes.join(","))).id("changes")
                    }),
                ])
                .spacing(10.0)
                .fill_width(),
            )
    }
}
