use zintl_ui_desktop::*;

/// Two one-column grids of rows, one striped and one plain.
pub struct MainView;

impl MainView {
    /// Builds four rows whose IDs start with `prefix`.
    fn rows(prefix: &str) -> ListFactory<RenderNode> {
        (0..4)
            .map(|row| {
                ElementFactory::new(
                    Text::new(format!("Train {row}"))
                        .id(format!("{prefix}-{row}"))
                        .fill_width(),
                )
            })
            .collect()
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Window::new(Rect::new(100.0, 100.0, 360.0, 360.0), "Zintl Grid Variants UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    Grid::new(1, Self::rows("striped"))
                        .striped(true)
                        .spacing(0.0)
                        .fill_width(),
                    Grid::new(1, Self::rows("plain")).spacing(0.0).fill_width(),
                ])
                .spacing(24.0)
                .fill_width(),
            )
    }
}
