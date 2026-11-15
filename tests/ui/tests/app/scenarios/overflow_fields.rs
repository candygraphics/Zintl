use zintl_ui_desktop::*;

/// A 200-point pane holding three 300-point text fields in too little height, beside a pane with
/// a fourth field and labels echoing all four.
pub struct MainView {
    values: [Store<String>; 4],
}

impl MainView {
    pub fn new() -> Self {
        Self {
            values: [Store::default(); 4],
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.values = [(); 4].map(|_| cx.store(String::new()));
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let values = self.values;
        let field = |index: usize| {
            TextField::new()
                .id(format!("field-{}", index + 1))
                .bind(values[index])
        };
        // Each field asks for more width than the pane has, and together they need more height.
        let wide = |index: usize| field(index).minimum_size(Size::new(300.0, 0.0));
        let echoes = (0..4)
            .map(|index| {
                ElementFactory::new(cx.watch(values[index], move |value| {
                    Text::new(format!("{}:{value}", index + 1)).id(format!("value-{}", index + 1))
                }))
            })
            .collect::<Vec<_>>();
        Window::new(
            Rect::new(100.0, 100.0, 520.0, 110.0),
            "Zintl Overflow Fields UI Test",
        )
        .id("main-window")
        .content(
            SplitView::horizontal(
                VStack::new(list![wide(0), wide(1), wide(2)]).spacing(8.0),
                VStack::new(list![HStack::new(echoes).spacing(6.0), field(3)]).spacing(8.0),
            )
            .position(200.0),
        )
    }
}
