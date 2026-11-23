use zintl_ui_desktop::*;

/// Read-only text and integer fields beside editable ones, a button that changes their
/// values, and a count of the edits they report.
pub struct MainView {
    kind: Store<String>,
    total: Store<i64>,
    changes: Store<u32>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            kind: Store::default(),
            total: Store::default(),
            changes: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.kind = cx.store("Expense".to_owned());
        self.total = cx.store(1234);
        self.changes = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (kind, total, changes) = (self.kind, self.total, self.changes);
        let count = move |cx: &mut Context<'_>| cx.update(changes, |changes| *changes += 1);
        Window::new("Zintl Read-Only Field UI Test")
            .size(Size::new(360.0, 280.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    TextField::new()
                        .bind(kind)
                        .editable(false)
                        .on_change(move |cx, _| count(cx))
                        .on_commit(move |cx, _| count(cx))
                        .minimum_size(Size::new(200.0, 0.0))
                        .id("kind"),
                    IntegerField::new()
                        .bind(total)
                        .grouping(true)
                        .editable(false)
                        .on_change(move |cx, _| count(cx))
                        .minimum_size(Size::new(200.0, 0.0))
                        .id("total"),
                    TextField::new()
                        .minimum_size(Size::new(200.0, 0.0))
                        .id("note"),
                    IntegerField::new()
                        .minimum_size(Size::new(200.0, 0.0))
                        .id("count"),
                    Button::new("Change").id("change-values").on_click(move |cx| {
                        cx.update(kind, |kind| *kind = "Income".to_owned());
                        cx.update(total, |total| *total = 5678);
                    }),
                    cx.watch(changes, |changes| Text::new(format!("Changes: {changes}")).id("changes")),
                ])
                .spacing(8.0),
            )
    }
}
