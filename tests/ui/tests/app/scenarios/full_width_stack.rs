use zintl_ui_desktop::*;

pub struct MainView;

impl View for MainView {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Window::new("Zintl Full Width Stack UI Test").size(Size::new(640.0, 400.0))
        .id("main-window")
        .content(
            VStack::new(list![
                HStack::new(list![
                    TextField::new().id("space-leading").placeholder("Leading"),
                    TextField::new()
                        .id("space-trailing")
                        .placeholder("Trailing"),
                ])
                .spacing(24.0)
                .fill_width()
                .distribution(MainAxisDistribution::SpaceBetween),
                HStack::new(list![
                    TextField::new().id("equal-leading").placeholder("Leading"),
                    TextField::new()
                        .id("equal-trailing")
                        .placeholder("Trailing"),
                ])
                .spacing(24.0)
                .fill_width()
                .equal_children(),
            ])
            .spacing(24.0)
            .fill_width(),
        )
    }
}
