use zintl_ui_desktop::*;

/// A pull-down button, selectable row numbers, a spinner, and separators.
pub struct MainView {
    loaded: Store<String>,
    selected: Store<usize>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            loaded: Store::default(),
            selected: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.loaded = cx.store("Nothing loaded".to_owned());
        self.selected = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (loaded, selected) = (self.loaded, self.selected);
        let current = *cx.get(selected);
        let sample = |name: &'static str| {
            MenuItem::new(format!("Load {name}"))
                .on_select(move |cx| cx.update(loaded, |loaded| *loaded = format!("Loaded {name}")))
        };
        let row = |number: usize| {
            Button::new(number.to_string())
                .id(format!("row-{number}"))
                .selected(current == number)
                .on_click(move |cx| cx.update(selected, |selected| *selected = number))
        };
        Window::new(
            Rect::new(100.0, 100.0, 480.0, 320.0),
            "Zintl Controls UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                HStack::new(list![
                    Button::new("Samples")
                        .id("samples")
                        .menu(Menu::items([sample("A"), sample("B")])),
                    Separator::vertical(),
                    ProgressIndicator::spinning().id("spinner"),
                    Button::new("Problem")
                        .title_color(Color::rgba(0.9, 0.1, 0.1, 1.0))
                        .id("colored-button"),
                    Button::new("Plain").id("plain-button"),
                    ColorWell::new()
                        .color(Color::rgba(1.0, 0.0, 0.0, 1.0))
                        .id("declared-color"),
                ])
                .spacing(8.0),
                cx.watch(loaded, |loaded| Text::new(loaded.clone()).id("loaded")),
                Separator::horizontal(),
                HStack::new(list![row(1), row(2), row(3)]).spacing(4.0),
                Text::new(format!("Selected: {current}")).id("selected"),
            ])
            .spacing(12.0),
        )
    }
}
