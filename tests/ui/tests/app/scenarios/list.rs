use zintl_ui_desktop::*;

/// The rows, selected by this key rather than by their text.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Callsign {
    Alpha,
    Bravo,
    Charlie,
    Delta,
}

impl ItemKey for Callsign {
    fn item_id(&self) -> String {
        format!("{self:?}").to_lowercase()
    }
}

/// A sectioned list bound to a selection Store, with buttons that select and clear from the app.
pub struct MainView {
    selection: Store<Option<Callsign>>,
    picks: Store<Vec<String>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selection: Store::default(),
            picks: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.selection = cx.store(None);
        self.picks = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (selection, picks) = (self.selection, self.picks);
        let row = |key: Callsign| {
            ListRow::new(key, Text::new(format!("{key:?}")).id(format!("{}-title", key.item_id())))
        };
        Window::new("Zintl List UI Test").size(Size::new(420.0, 460.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    List::new([
                        ListSection::new([row(Callsign::Alpha)]).title("Pinned"),
                        ListSection::new([
                            row(Callsign::Bravo),
                            row(Callsign::Charlie),
                            row(Callsign::Delta),
                        ])
                        .title("Recent"),
                    ])
                    .bind(selection)
                    .id("list")
                    .on_select(move |cx, key| {
                        let id = key.map_or("none".to_owned(), ItemKey::item_id);
                        cx.update(picks, |picks| picks.push(id));
                    }),
                    HStack::new(list![
                        Button::new("Select Charlie").id("select-charlie").on_click(move |cx| {
                            cx.update(selection, |selection| *selection = Some(Callsign::Charlie));
                        }),
                        Button::new("Clear").id("clear").on_click(move |cx| {
                            cx.update(selection, |selection| *selection = None);
                        }),
                    ])
                    .spacing(8.0),
                    cx.watch(selection, |selection| {
                        let id = selection.map_or("none".to_owned(), |key| key.item_id());
                        Text::new(format!("Selected: {id}"))
                            .id("selection-value")
                    }),
                    cx.watch(picks, |picks| {
                        Text::new(format!("Picks: {}", picks.join(","))).id("picks")
                    }),
                ])
                .spacing(10.0)
                .fill_width(),
            )
    }
}
