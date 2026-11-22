use zintl_ui_desktop::*;

/// Labels that differ only in font, color, or line limit.
pub struct MainView;

impl View for MainView {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let long = "The limited express leaves Tokyo at nine and calls at Shinagawa, \
                    Yokohama, Odawara, Atami, Mishima, and Shizuoka before Nagoya.";
        Window::new("Zintl Text Styles UI Test").size(Size::new(360.0, 480.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    Text::new("Platform 7").font(Font::system(13.0)).id("small"),
                    Text::new("Platform 7").font(Font::system(28.0)).id("large"),
                    Text::new("Platform 7").font(Font::system(28.0).bold()).id("large-bold"),
                    Text::new("Delayed")
                        .font(Font::system(28.0).bold())
                        .color(TextColor::Custom(Color::rgb(1.0, 0.0, 0.0)))
                        .id("red"),
                    Text::new("Delayed")
                        .font(Font::system(28.0).bold())
                        .color(TextColor::System(SystemColor::Red))
                        .id("system-red"),
                    Text::new(long).line_limit(1).fill_width().id("one-line"),
                    Text::new(long).line_limit(2).fill_width().id("two-lines"),
                ])
                .spacing(8.0)
                .fill_width(),
            )
    }
}
