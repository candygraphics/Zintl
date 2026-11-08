use zintl_ui_desktop::*;

/// Views described for VoiceOver: a symbol-only button, a slider named by a label,
/// a named group whose value follows a Store, hidden decoration, and a disclosure group.
pub struct MainView {
    tools: Store<usize>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            tools: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.tools = cx.store(2);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let tools = self.tools;
        let count = *cx.get(tools);
        Window::new(Rect::new(100.0, 100.0, 420.0, 360.0), "Zintl Accessibility UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    HStack::new(list![
                        Button::new("")
                            .system_image("plus")
                            .id("add")
                            .accessibility_label("Add tool")
                            .accessibility_hint("Adds a tool to the toolbox")
                            .on_click(move |cx| {
                                cx.update(tools, |tools| *tools += 1);
                                cx.announce("Tool added", AnnouncementPriority::Medium);
                            }),
                        Text::new("★").id("decoration").accessibility_hidden(),
                    ])
                    .spacing(8.0)
                    .id("toolbox")
                    .accessibility_label("Toolbox")
                    .accessibility_value(format!("{count} tools")),
                    HStack::new(list![
                        Text::new("Speed").id("speed-label"),
                        Slider::new(0.0..=100.0)
                            .id("speed")
                            .accessibility_labeled_by("speed-label"),
                    ])
                    .spacing(8.0),
                    DisclosureGroup::new("Line settings", Text::new("Details")).id("line-settings"),
                ])
                .spacing(16.0)
                .padding(16.0),
            )
    }
}
