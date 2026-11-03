use zintl_ui_desktop::*;

/// Formats metres as kilometre posts, such as `12k345.67`.
fn format_km(value: f64) -> String {
    let km = (value / 1000.0).floor();
    format!("{km}k{:06.2}", value - km * 1000.0)
}

/// Reads `12k345.67` and plain `12345.67`.
fn parse_km(text: &str) -> Option<f64> {
    match text.split_once('k') {
        Some((km, m)) => Some(km.parse::<f64>().ok()? * 1000.0 + m.parse::<f64>().ok()?),
        None => text.parse().ok(),
    }
}

pub struct MainView {
    name: Store<String>,
    commits: Store<Vec<String>>,
    position: Store<f64>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            name: Store::default(),
            commits: Store::default(),
            position: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.name = cx.store(String::new());
        self.commits = cx.store(Vec::new());
        self.position = cx.store(1000.0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let commits = self.commits;
        Window::new(
            Rect::new(100.0, 100.0, 480.0, 320.0),
            "Zintl Text Commit UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                TextField::new()
                    .id("name-field")
                    .placeholder("Name")
                    .bind(self.name)
                    .on_commit(move |cx, text| {
                        let text = text.to_owned();
                        cx.update(commits, |commits| commits.push(text));
                    }),
                cx.watch(commits, |commits| {
                    Text::new(format!("Commits: {}", commits.join("|"))).id("commits")
                }),
                NumberField::new()
                    .formatter(format_km)
                    .parser(parse_km)
                    .suffix(" m")
                    .range(0.0..=100_000.0)
                    .step(100.0)
                    .bind(self.position),
                cx.watch(self.position, |position| {
                    Text::new(format!("Position: {position}")).id("position")
                }),
            ])
            .spacing(12.0),
        )
    }
}
