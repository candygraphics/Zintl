use zintl_ui_desktop::*;

/// One picker per style, a disabled picker, and a button that selects the last option everywhere.
pub struct MainView {
    menu: Store<usize>,
    segmented: Store<usize>,
    radio: Store<usize>,
    changes: Store<Vec<usize>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            menu: Store::default(),
            segmented: Store::default(),
            radio: Store::default(),
            changes: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.menu = cx.store(0);
        self.segmented = cx.store(0);
        self.radio = cx.store(0);
        self.changes = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (menu, segmented, radio, changes) = (self.menu, self.segmented, self.radio, self.changes);
        let options = ["Local", "Rapid", "Express"];
        let echo = |cx: &mut Context<'_>, store: Store<usize>, id: &'static str| {
            cx.watch(store, move |index| Text::new(options[*index]).id(id))
        };
        Window::new(Rect::new(100.0, 100.0, 420.0, 420.0), "Zintl Pickers UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    Picker::new(options)
                        .bind(menu)
                        .id("menu-picker")
                        .on_change(move |cx, index| cx.update(changes, |changes| changes.push(index))),
                    Picker::new(options)
                        .style(PickerStyle::Segmented)
                        .bind(segmented)
                        .id("segmented-picker"),
                    Picker::new(options)
                        .style(PickerStyle::Radio)
                        .bind(radio)
                        .id("radio-picker"),
                    Picker::new(options)
                        .selected(1)
                        .enabled(false)
                        .id("disabled-picker"),
                    Button::new("Select Express").id("select-last").on_click(move |cx| {
                        for store in [menu, segmented, radio] {
                            cx.update(store, |index| *index = 2);
                        }
                    }),
                    echo(cx, menu, "menu-value"),
                    echo(cx, segmented, "segmented-value"),
                    echo(cx, radio, "radio-value"),
                    cx.watch(changes, |changes| {
                        let changes = changes.iter().map(usize::to_string).collect::<Vec<_>>();
                        Text::new(format!("Changes: {}", changes.join(","))).id("changes")
                    }),
                ])
                .spacing(10.0),
            )
    }
}
