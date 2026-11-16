use zintl_ui_desktop::*;

/// A slider, a tab view, and a color well that keep the app's value and only count the attempts
/// to change them.
pub struct MainView {
    slider: Store<usize>,
    tabs: Store<usize>,
    well: Store<usize>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            slider: Store::default(),
            tabs: Store::default(),
            well: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.slider = cx.store(0);
        self.tabs = cx.store(0);
        self.well = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (slider, tabs, well) = (self.slider, self.tabs, self.well);
        let rejected = |cx: &mut Context<'_>, store: Store<usize>, id: &'static str| {
            cx.watch(store, move |count| Text::new(format!("Rejected: {count}")).id(id))
        };
        Window::new(Rect::new(100.0, 100.0, 420.0, 420.0), "Zintl Fixed Values UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    Slider::new(0.0..=100.0)
                        .value(25.0)
                        .id("slider")
                        .minimum_size(Size::new(200.0, 24.0))
                        .on_change(move |cx, _| cx.update(slider, |count| *count += 1)),
                    rejected(cx, slider, "slider-rejected"),
                    ColorWell::new()
                        .value(Color::rgb(1.0, 0.0, 0.0))
                        .id("well")
                        .on_change(move |cx, _| cx.update(well, |count| *count += 1)),
                    rejected(cx, well, "well-rejected"),
                    TabView::new([
                        Tab::new("First", Text::new("First page").id("first-page")),
                        Tab::new("Second", Text::new("Second page").id("second-page")),
                    ])
                    .selection(0)
                    .id("tabs")
                    .on_change(move |cx, _| cx.update(tabs, |count| *count += 1)),
                    rejected(cx, tabs, "tabs-rejected"),
                ])
                .spacing(10.0),
            )
    }
}
