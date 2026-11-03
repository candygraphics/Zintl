use zintl_ui_desktop::*;

/// Content wider than its pane: a toolbar row and a grid in a two-axis scroll view.
pub struct MainView;

impl View for MainView {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let toolbar = HStack::new(
            (1..=5)
                .map(|index| ElementFactory::new(Button::new(format!("Toolbar action {index}"))))
                .collect::<Vec<_>>(),
        )
        .spacing(4.0);
        let pickers = |row: u32| {
            HStack::new(list![
                Picker::new(["A long first option"]).id(format!("first-picker-{row}")),
                Picker::new(["A long second option"]).id(format!("second-picker-{row}")),
            ])
            .spacing(4.0)
        };
        let grid = Grid::new(
            3,
            list![
                Text::new("Row 1"),
                pickers(1),
                Text::new("After 1").id("after-1"),
                Text::new("Row 2"),
                pickers(2),
                Text::new("After 2").id("after-2"),
            ],
        )
        .spacing(6.0);
        let detail = VStack::new(list![
            Text::new("Detail").id("detail"),
            toolbar,
            ScrollView::new(VStack::new(list![grid]).padding(8.0))
                .axes(ScrollAxes {
                    horizontal: true,
                    vertical: true,
                })
                .id("table-scroll"),
        ])
        .grow();
        Window::new(
            Rect::new(100.0, 100.0, 520.0, 320.0),
            "Zintl Overflow UI Test",
        )
        .id("main-window")
        .content(
            SplitView::horizontal(VStack::new(list![Text::new("List")]).grow(), detail)
                .position(200.0),
        )
    }
}
