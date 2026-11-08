use zintl_ui_desktop::*;

/// A progress bar the app advances, separators between labelled blocks, and a spacer.
pub struct MainView {
    progress: Store<f64>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            progress: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.progress = cx.store(0.25);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let progress = self.progress;
        Window::new(Rect::new(100.0, 100.0, 480.0, 260.0), "Zintl Indicators UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    cx.watch(progress, |value| {
                        ProgressIndicator::bar(*value).id("progress")
                    }),
                    Button::new("Advance").id("advance").on_click(move |cx| {
                        cx.update(progress, |value| *value = (*value + 0.5).min(1.0));
                    }),
                    Text::new("Above").id("above"),
                    Separator::horizontal(),
                    Text::new("Below").id("below"),
                    HStack::new(list![
                        Text::new("Left").id("left"),
                        Separator::vertical(),
                        Text::new("Middle").id("middle"),
                        Text::new("Extra").id("extra"),
                        Spacer::new(),
                        Text::new("Right").id("right"),
                    ])
                    .spacing(8.0)
                    .fill_width(),
                ])
                .spacing(8.0)
                .fill_width(),
            )
    }
}
