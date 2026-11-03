use zintl_ui_desktop::*;

/// A window with an undoable edit count that asks before closing.
pub struct MainView {
    done: Store<u32>,
    undone: Store<u32>,
    allow_close: Store<bool>,
    requests: Store<u32>,
    note: Store<String>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            done: Store::default(),
            undone: Store::default(),
            allow_close: Store::default(),
            requests: Store::default(),
            note: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.done = cx.store(0);
        self.undone = cx.store(0);
        self.allow_close = cx.store(false);
        self.requests = cx.store(0);
        self.note = cx.store(String::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (done, undone) = (self.done, self.undone);
        let (allow_close, requests) = (self.allow_close, self.requests);
        let edits = *cx.get(done);
        let redos = *cx.get(undone);
        Window::new(
            Rect::new(100.0, 100.0, 480.0, 320.0),
            "Zintl Document UI Test",
        )
        .id("main-window")
        .edited(edits > 0)
        .on_close_request(move |cx| {
            cx.update(requests, |requests| *requests += 1);
            *cx.get(allow_close)
        })
        .undo(
            UndoHandler::new(edits > 0, redos > 0)
                .on_undo(move |cx| {
                    cx.update(done, |done| *done -= 1);
                    cx.update(undone, |undone| *undone += 1);
                })
                .on_redo(move |cx| {
                    cx.update(undone, |undone| *undone -= 1);
                    cx.update(done, |done| *done += 1);
                }),
        )
        .content(
            VStack::new(list![
                Text::new(format!("Edits: {edits}")).id("edits"),
                Button::new("Edit").id("edit-button").on_click(move |cx| {
                    cx.update(done, |done| *done += 1);
                    cx.update(undone, |undone| *undone = 0);
                }),
                Button::new("Allow closing")
                    .id("allow-close")
                    .on_click(move |cx| cx.update(allow_close, |allow| *allow = true)),
                cx.watch(requests, |requests| {
                    Text::new(format!("Close requests: {requests}")).id("close-requests")
                }),
                TextField::new()
                    .id("note-field")
                    .placeholder("Note")
                    .bind(self.note),
            ])
            .spacing(12.0),
        )
    }
}
