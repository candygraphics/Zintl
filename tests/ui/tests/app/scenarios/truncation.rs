use zintl_ui_desktop::*;

/// Line-limited labels in rows: one too long for its row, one that fits.
pub struct MainView;

impl View for MainView {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let long = "A very long document name that cannot fit beside the toolbar buttons.route";
        Window::new(
            Rect::new(100.0, 100.0, 360.0, 200.0),
            "Zintl Truncation UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                HStack::new(list![
                    Button::new("Save").id("save"),
                    Text::new(long).line_limit(1).id("long-label"),
                    Button::new("Reload").id("reload"),
                ])
                .spacing(4.0),
                HStack::new(list![
                    Text::new("Short label").line_limit(1).id("short-label"),
                    Spacer::new(),
                ]),
            ])
            .spacing(8.0)
            .padding(8.0),
        )
    }
}
