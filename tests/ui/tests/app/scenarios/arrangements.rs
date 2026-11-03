use zintl_ui_desktop::*;

/// An overlay label, a striped grid, a folding section, and a list that jumps to a row.
pub struct MainView {
    target: Store<Option<String>>,
    expanded: Store<bool>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            target: Store::default(),
            expanded: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.target = cx.store(None);
        self.expanded = cx.store(false);
    }

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let target = self.target;
        let rows = (1..=40)
            .map(|row| {
                ElementFactory::new(
                    Text::new(format!("Row {row}"))
                        .id(format!("row-{row}"))
                        .fill_width(),
                )
            })
            .collect::<Vec<_>>();
        let cells = [
            "Name",
            "Length",
            "Section A",
            "1k200",
            "Section B",
            "15k000",
        ]
        .into_iter()
        .enumerate()
        .map(|(index, text)| ElementFactory::new(Text::new(text).id(format!("cell-{index}"))))
        .collect::<Vec<_>>();
        Window::new(
            Rect::new(100.0, 100.0, 520.0, 520.0),
            "Zintl Arrangements UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                    ZStack::new(list![
                        Text::new("Preview area").id("preview"),
                        Text::new("Updating").id("overlay"),
                    ]),
                    Grid::new(2, cells).striped(true).spacing(8.0).id("grid"),
                    DisclosureGroup::new("Line settings", Text::new("Gauge 1067").id("details"))
                        .bind(self.expanded)
                        .id("disclosure"),
                    Button::new("Jump to 30").id("jump").on_click(
                        move |cx| cx.update(target, |target| *target = Some("row-30".into()))
                    ),
                    ScrollView::new(VStack::new(rows).spacing(4.0).fill_width())
                        .scroll_to(target, ScrollAlignment::Center)
                        .height(160.0),
                ])
            .spacing(10.0)
            .fill_width(),
        )
    }
}
