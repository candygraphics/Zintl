use zintl_ui_desktop::*;

#[derive(Default)]
pub struct MainView {
    shown: Store<bool>,
    scroll: Store<ScrollStatus>,
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.shown = cx.store(true);
        self.scroll = cx.store(ScrollStatus::default());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let shown = self.shown;
        // Hiding the list removes the native scroll view, so showing it creates a new one.
        let list = if *cx.get(shown) {
            let rows = (1..=60)
                .map(|row| {
                    ElementFactory::new(Text::new(format!("Row {row}")).id(format!("row-{row}")))
                })
                .collect::<ListFactory<RenderNode>>();
            ElementFactory::new(
                ScrollView::new(VStack::new(rows).spacing(6.0).padding(8.0))
                    .id("rows")
                    .bind(self.scroll),
            )
        } else {
            ElementFactory::new(Text::new("Hidden").id("hidden"))
        };
        let offset = cx.get(self.scroll).y;
        Window::new(
            Rect::new(100.0, 100.0, 480.0, 400.0),
            "Zintl Scroll Status UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                Button::new("Toggle List")
                    .id("toggle-list")
                    .on_click(move |cx| cx.update(shown, |shown| *shown = !*shown)),
                Text::new(format!("Offset: {offset:.0}"))
                    .id("scroll-offset")
                    .minimum_size(Size::new(200.0, 20.0)),
                list,
            ])
            .spacing(8.0)
            .padding(16.0)
            .grow(),
        )
    }
}
