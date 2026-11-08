use zintl_ui_desktop::*;

/// Tooltips on an enabled button, a disabled button, and a container, beside a button without one.
pub struct MainView;

impl View for MainView {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Window::new(Rect::new(100.0, 100.0, 420.0, 300.0), "Zintl Help UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    Button::new("Save").id("save").help("Saves the timetable"),
                    Button::new("Reset").id("reset"),
                    Button::new("Publish")
                        .id("publish")
                        .enabled(false)
                        .help("Save the timetable first"),
                    VStack::new(list![Text::new("Depot").id("depot")])
                        .padding(16.0)
                        .help("Trains rest here overnight"),
                ])
                .spacing(16.0),
            )
    }
}
