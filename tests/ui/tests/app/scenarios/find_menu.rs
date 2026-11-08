use zintl_ui_desktop::*;

/// A text editor under the standard menus, whose Find items open its find bar.
pub struct MainView {
    text: Store<AttributedText>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            text: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.text = cx.store(AttributedText::from("Tokyo Shinagawa Yokohama"));
    }

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        list![
            MainMenu::standard().app_name("Zintl Find"),
            Window::new(Rect::new(100.0, 100.0, 480.0, 320.0), "Zintl Find Menu UI Test")
                .id("main-window")
                .content(
                    VStack::new(list![TextEditor::new().bind(self.text)])
                        .fill_width()
                        .grow(),
                ),
        ]
    }
}
