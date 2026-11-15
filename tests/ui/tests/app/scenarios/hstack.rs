use zintl_ui_desktop::*;

pub struct MainView;

impl View for MainView {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Window::new(
            Rect::new(100.0, 100.0, 640.0, 400.0),
            "Zintl HStack UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                HStack::new(list![
                    TextField::new().id("leading-field").placeholder("Leading"),
                    TextField::new()
                        .id("trailing-field")
                        .placeholder("Trailing"),
                ])
                .id("field-row")
                .spacing(24.0),
                // No alignment is declared, so the row centers the field beside the taller button.
                HStack::new(list![
                    TextField::new()
                        .id("centered-field")
                        .minimum_size(Size::new(160.0, 0.0)),
                    Button::new("Go").id("centered-button"),
                ])
                .spacing(8.0),
            ])
            .spacing(12.0),
        )
    }
}
