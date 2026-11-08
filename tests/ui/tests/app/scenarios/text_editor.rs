use zintl_ui_desktop::*;

/// A bound rich text editor that echoes its text and counts on_change calls.
pub struct MainView {
    text: Store<AttributedText>,
    changes: Store<u32>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            text: Store::default(),
            changes: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.text = cx.store(AttributedText::new());
        self.changes = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (text, changes) = (self.text, self.changes);
        let bold = SpanStyle {
            bold: true,
            ..SpanStyle::default()
        };
        list![
            MainMenu::standard().app_name("Zintl Text Editor"),
            Window::new(Rect::new(100.0, 100.0, 520.0, 380.0), "Zintl Text Editor UI Test")
                .id("main-window")
                .content(
                    VStack::new(list![
                        cx.watch(text, |text| Text::new(format!("Text: {}", text.text()))
                            .id("text-value")),
                        cx.watch(text, |text| {
                            let bold = text.runs().filter(|(_, style)| style.bold).count();
                            Text::new(format!("Bold runs: {bold}")).id("bold-runs")
                        }),
                        cx.watch(changes, |changes| Text::new(format!("Changes: {changes}"))
                            .id("changes")),
                        Button::new("Load Timetable").id("load").on_click(move |cx| {
                            let loaded = AttributedText::from_segments([
                                ("Timetable\n", SpanStyle::default(), ParagraphStyle::Title),
                                ("Departs ", SpanStyle::default(), ParagraphStyle::Body),
                                ("9:00", bold.clone(), ParagraphStyle::Body),
                            ]);
                            cx.update(text, |text| *text = loaded);
                        }),
                        TextEditor::new()
                            .bind(text)
                            .on_change(move |cx| cx.update(changes, |changes| *changes += 1)),
                    ])
                    .spacing(8.0)
                    .fill_width()
                    .grow(),
                ),
        ]
    }
}
