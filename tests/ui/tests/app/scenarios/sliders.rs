use zintl_ui_desktop::*;

/// A free slider, a slider that snaps to ticks, a full-width slider, and a disabled one.
///
/// Each slider sits in a labelled row, since a vertical stack stretches its children.
pub struct MainView {
    free: Store<f64>,
    snapping: Store<f64>,
    changes: Store<usize>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            free: Store::default(),
            snapping: Store::default(),
            changes: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.free = cx.store(0.0);
        self.snapping = cx.store(0.0);
        self.changes = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (free, snapping, changes) = (self.free, self.snapping, self.changes);
        Window::new(Rect::new(100.0, 100.0, 420.0, 340.0), "Zintl Sliders UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    HStack::new(list![
                        Text::new("Free"),
                        Slider::new(0.0..=100.0)
                            .bind(free)
                            .id("free-slider")
                            .minimum_size(Size::new(200.0, 24.0))
                            .on_change(move |cx, _| cx.update(changes, |changes| *changes += 1)),
                    ]),
                    HStack::new(list![
                        Text::new("Snapping"),
                        Slider::new(0.0..=100.0)
                            .snapping_ticks(5)
                            .bind(snapping)
                            .id("snapping-slider")
                            .minimum_size(Size::new(200.0, 24.0)),
                    ]),
                    HStack::new(list![
                        Text::new("Wide"),
                        Slider::new(0.0..=1.0).fill_width().id("wide-slider"),
                    ]),
                    HStack::new(list![
                        Text::new("Disabled"),
                        Slider::new(0.0..=100.0)
                            .enabled(false)
                            .id("disabled-slider")
                            .minimum_size(Size::new(200.0, 24.0)),
                    ]),
                    Button::new("Set 75").id("set-75").on_click(move |cx| {
                        cx.update(free, |value| *value = 75.0);
                        cx.update(snapping, |value| *value = 75.0);
                    }),
                    cx.watch(free, |value| Text::new(format!("{value:.0}")).id("free-value")),
                    cx.watch(snapping, |value| {
                        Text::new(format!("{value:.2}")).id("snapping-value")
                    }),
                    cx.watch(changes, |changes| {
                        Text::new(if *changes > 0 { "Changed" } else { "Unchanged" }).id("changes")
                    }),
                ])
                .spacing(10.0)
                .fill_width(),
            )
    }
}
