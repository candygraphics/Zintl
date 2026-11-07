use zintl_ui_desktop::*;

/// A combo box, a search field, and a clamped integer field, with a button that sets all three.
pub struct MainView {
    station: Store<String>,
    query: Store<String>,
    count: Store<f64>,
    searches: Store<Vec<String>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            station: Store::default(),
            query: Store::default(),
            count: Store::default(),
            searches: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.station = cx.store(String::new());
        self.query = cx.store(String::new());
        self.count = cx.store(3.0);
        self.searches = cx.store(Vec::new());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (station, query, count, searches) =
            (self.station, self.query, self.count, self.searches);
        Window::new(Rect::new(100.0, 100.0, 420.0, 320.0), "Zintl Text Inputs UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    ComboBox::new(["Tokyo", "Osaka", "Nagoya"])
                        .bind(station)
                        .placeholder("Station")
                        .id("combo")
                        .minimum_size(Size::new(200.0, 24.0)),
                    SearchField::new()
                        .bind(query)
                        .placeholder("Search lines")
                        .id("search")
                        .minimum_size(Size::new(200.0, 24.0))
                        .on_change(move |cx, text| cx.update(searches, |searches| searches.push(text))),
                    NumberField::new()
                        .range(0.0..=10.0)
                        .step(1.0)
                        .fraction_digits(0)
                        .bind(count)
                        .id("count"),
                    Button::new("Reset").id("reset").on_click(move |cx| {
                        cx.update(station, |station| *station = "Nagoya".into());
                        cx.update(query, |query| *query = "rail".into());
                        cx.update(count, |count| *count = 5.0);
                    }),
                    cx.watch(station, |station| {
                        Text::new(format!("Station: {station}")).id("station-value")
                    }),
                    cx.watch(query, |query| Text::new(format!("Query: {query}")).id("query-value")),
                    cx.watch(count, |count| Text::new(format!("Count: {count}")).id("count-value")),
                    cx.watch(searches, |searches| {
                        Text::new(format!("Last search: {}", searches.last().map_or("", String::as_str)))
                            .id("last-search")
                    }),
                ])
                .spacing(10.0),
            )
    }
}
