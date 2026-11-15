use zintl_ui_desktop::*;

/// One picker per style, a disabled picker, pickers that reject changes, and a button that
/// selects the last option everywhere.
pub struct MainView {
    menu: Store<usize>,
    segmented: Store<usize>,
    radio: Store<usize>,
    changes: Store<Vec<usize>>,
    rejected: Store<usize>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            menu: Store::default(),
            segmented: Store::default(),
            radio: Store::default(),
            changes: Store::default(),
            rejected: Store::default(),
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
        self.rejected = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (menu, segmented, radio, changes, rejected) = (
            self.menu,
            self.segmented,
            self.radio,
            self.changes,
            self.rejected,
        );
        let reject = move |cx: &mut Context<'_>, _: usize| cx.update(rejected, |count| *count += 1);
        let options = ["Local", "Rapid", "Express"];
        let echo = |cx: &mut Context<'_>, store: Store<usize>, id: &'static str| {
            cx.watch(store, move |index| Text::new(options[*index]).id(id))
        };
        Window::new(Rect::new(100.0, 100.0, 420.0, 560.0), "Zintl Pickers UI Test")
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
                    // The app keeps these pickers on their first option and only counts the attempts.
                    Picker::new(["Fixed", "Other"])
                        .selected(0)
                        .id("fixed-picker")
                        .on_change(reject),
                    Picker::new(["North", "South"])
                        .style(PickerStyle::Radio)
                        .selected(0)
                        .on_change(reject),
                    cx.watch(rejected, |count| {
                        Text::new(format!("Rejected: {count}")).id("rejected")
                    }),
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
