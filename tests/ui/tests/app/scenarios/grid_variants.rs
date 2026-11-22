use zintl_ui_desktop::*;

/// Two one-column grids of rows, one striped and one plain, and a four-column grid whose heading
/// spans every column between two groups of rows.
pub struct MainView;

impl MainView {
    /// Builds one row of four cells whose IDs start with `prefix`; each cell fills its column.
    fn cells(prefix: &str, titles: [&str; 4]) -> ListFactory<RenderNode> {
        titles
            .iter()
            .enumerate()
            .map(|(column, title)| {
                ElementFactory::new(
                    Text::new(*title)
                        .id(format!("{prefix}-{column}"))
                        .fill_width(),
                )
            })
            .collect()
    }

    /// Builds four one-cell rows whose IDs start with `prefix`.
    fn rows(prefix: &str) -> Vec<GridRow> {
        (0..4)
            .map(|row| {
                GridRow::new(list![
                    Text::new(format!("Train {row}"))
                        .id(format!("{prefix}-{row}"))
                        .fill_width(),
                ])
            })
            .collect()
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Window::new("Zintl Grid Variants UI Test").size(Size::new(360.0, 420.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    Grid::rows(1, Self::rows("striped"))
                        .striped(true)
                        .spacing(0.0)
                        .fill_width(),
                    Grid::rows(1, Self::rows("plain")).spacing(0.0).fill_width(),
                    Grid::new(4, {
                        let mut children = Self::cells("food", ["Food", "Bar", "3480", "62%"]);
                        children.push(ElementFactory::new(
                            GridCell::new(Text::new("Transport").id("heading").fill_width())
                                .column_span(4),
                        ));
                        children.extend(Self::cells("bus", ["Bus pass", "Bar", "1200", "21%"]));
                        children
                    })
                    .spacing(6.0)
                    .fill_width(),
                ])
                .spacing(24.0)
                .fill_width(),
            )
    }
}
