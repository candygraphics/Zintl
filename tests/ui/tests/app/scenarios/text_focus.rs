use zintl_ui_desktop::*;

/// Two fields whose second one takes focus on request, and a button that adds a third field and
/// requests focus for it in the same update.
pub struct MainView {
    values: [Store<String>; 3],
    focus_second: Trigger,
    focus_added: Trigger,
    added: Store<bool>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            values: [Store::default(); 3],
            focus_second: Trigger::default(),
            focus_added: Trigger::default(),
            added: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.values = [(); 3].map(|_| cx.store(String::new()));
        self.focus_second = cx.trigger();
        self.focus_added = cx.trigger();
        self.added = cx.store(false);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (values, focus_second, focus_added, added) =
            (self.values, self.focus_second, self.focus_added, self.added);
        let echo = |cx: &mut Context<'_>, index: usize, id: &'static str| {
            cx.watch(values[index], move |value| Text::new(format!("{id}: {value}")).id(id))
        };
        let added_field = if *cx.get(added) {
            ElementFactory::new(
                TextField::new()
                    .id("added-field")
                    .bind(values[2])
                    .focus_on(focus_added),
            )
        } else {
            ElementFactory::new(Text::new("No added field").id("no-added-field"))
        };
        Window::new(Rect::new(100.0, 100.0, 420.0, 360.0), "Zintl Text Focus UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    TextField::new().id("first-field").bind(values[0]),
                    TextField::new()
                        .id("second-field")
                        .bind(values[1])
                        .focus_on(focus_second),
                    added_field,
                    HStack::new(list![
                        Button::new("Focus second")
                            .id("focus-second")
                            .on_click(move |cx| cx.fire(focus_second, ())),
                        // Shows the field and requests its focus in one update.
                        Button::new("Add field").id("add-field").on_click(move |cx| {
                            cx.update(added, |added| *added = true);
                            cx.fire(focus_added, ());
                        }),
                    ])
                    .spacing(8.0),
                    echo(cx, 0, "first"),
                    echo(cx, 1, "second"),
                    echo(cx, 2, "added"),
                ])
                .spacing(10.0),
            )
    }
}
