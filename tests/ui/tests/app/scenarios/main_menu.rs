use zintl_ui_desktop::*;

/// The standard menus with custom items, optionally translated to Japanese.
pub struct MainView {
    japanese: bool,
    log: Store<Vec<String>>,
    route_enabled: Store<bool>,
}

impl MainView {
    pub fn new(japanese: bool) -> Self {
        Self {
            japanese,
            log: Store::default(),
            route_enabled: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.log = cx.store(Vec::new());
        self.route_enabled = cx.store(false);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (log, route_enabled) = (self.log, self.route_enabled);
        let report = move |entry: &'static str| {
            move |cx: &mut Context<'_>| cx.update(log, |log| log.push(entry.into()))
        };
        let menu = MainMenu::standard()
            .app_name("Zintl Menus")
            .settings(report("Settings"))
            .extend(
                StandardMenu::File,
                [MenuItem::new("Duplicate Route")
                    .shortcut(Shortcut::command("d"))
                    .on_select(report("Duplicate"))],
            )
            .menu(Menu::new(
                "Route",
                [
                    MenuItem::new("Say Hello")
                        .shortcut(Shortcut::command("h").shift())
                        .on_select(report("Hello")),
                    MenuItem::new("Reverse Route")
                        .enabled(*cx.get(route_enabled))
                        .on_select(report("Reverse")),
                    MenuItem::new("No Action"),
                ],
            ));
        let menu = if self.japanese {
            menu.localizer(JapaneseMenuLocalizer)
        } else {
            menu
        };
        list![
            menu,
            Window::new(Rect::new(100.0, 100.0, 420.0, 240.0), "Zintl Main Menu UI Test")
                .id("main-window")
                .content(
                    VStack::new(list![
                        cx.watch(log, |log| Text::new(format!("Log: {}", log.join(",")))
                            .id("log")),
                        Button::new("Enable Reverse")
                            .id("enable-reverse")
                            .on_click(move |cx| cx.update(route_enabled, |enabled| *enabled = true)),
                    ])
                    .spacing(10.0),
                ),
        ]
    }
}
