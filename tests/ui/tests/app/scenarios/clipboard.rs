use zintl_ui_desktop::*;

/// A computed result with a button that copies it, and a button that reads the clipboard into a Store.
pub struct MainView {
    pasted: Store<Option<String>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            pasted: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.pasted = cx.store(None);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let pasted = self.pasted;
        Window::new("Zintl Clipboard UI Test")
            .size(Size::new(360.0, 200.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    Text::new("1,234.50").id("result"),
                    Button::new("Copy")
                        .id("copy-result")
                        .on_click(|cx| cx.clipboard().set_text("1,234.50")),
                    Button::new("Paste").id("paste").on_click(move |cx| {
                        let text = cx.clipboard().text();
                        cx.update(pasted, |pasted| *pasted = text);
                    }),
                    cx.watch(pasted, |pasted| {
                        let pasted = pasted.as_deref().unwrap_or("none");
                        Text::new(format!("Pasted: {pasted}")).id("pasted-value")
                    }),
                ])
                .spacing(8.0),
            )
    }
}
