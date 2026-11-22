use zintl_ui_desktop::*;

/// A delivery service, which every picker selects by value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Service {
    Local,
    Rapid,
    Express,
}

impl Service {
    const ALL: [Service; 3] = [Service::Local, Service::Rapid, Service::Express];

    /// The options of a picker over every service, labeled with their names.
    fn options() -> [(Service, String); 3] {
        Self::ALL.map(|service| (service, format!("{service:?}")))
    }
}

/// One picker per style, a disabled picker, pickers that reject changes, pickers that start with
/// no selection, and buttons that write the Stores from the app.
pub struct MainView {
    menu: Store<Service>,
    segmented: Store<Service>,
    radio: Store<Service>,
    changes: Store<Vec<Service>>,
    rejected: Store<usize>,
    unset: Store<Option<Service>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            menu: Store::default(),
            segmented: Store::default(),
            radio: Store::default(),
            changes: Store::default(),
            rejected: Store::default(),
            unset: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.menu = cx.store(Service::Local);
        self.segmented = cx.store(Service::Local);
        self.radio = cx.store(Service::Local);
        self.changes = cx.store(Vec::new());
        self.rejected = cx.store(0);
        self.unset = cx.store(None);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (menu, segmented, radio, changes, rejected, unset) = (
            self.menu,
            self.segmented,
            self.radio,
            self.changes,
            self.rejected,
            self.unset,
        );
        let reject = move |cx: &mut Context<'_>, _: &usize| cx.update(rejected, |count| *count += 1);
        let echo = |cx: &mut Context<'_>, store: Store<Service>, id: &'static str| {
            cx.watch(store, move |service| Text::new(format!("{service:?}")).id(id))
        };
        // The options all hold a service, so `None` matches none of them.
        let unset_options = || Service::options().map(|(service, label)| (Some(service), label));
        Window::new("Zintl Pickers UI Test").size(Size::new(420.0, 680.0))
            .id("main-window")
            .content(
                VStack::new(list![
                    Picker::new(Service::options())
                        .bind(menu)
                        .id("menu-picker")
                        .on_change(move |cx, service| {
                            cx.update(changes, |changes| changes.push(*service))
                        }),
                    Picker::new(Service::options())
                        .style(PickerStyle::Segmented)
                        .bind(segmented)
                        .id("segmented-picker"),
                    Picker::new(Service::options())
                        .style(PickerStyle::Radio)
                        .bind(radio)
                        .id("radio-picker"),
                    Picker::new(Service::options())
                        .selection(Service::Rapid)
                        .enabled(false)
                        .id("disabled-picker"),
                    // The app keeps these pickers on their first option and only counts the attempts.
                    Picker::indexed(["Fixed", "Other"])
                        .selection(0)
                        .id("fixed-picker")
                        .on_change(reject),
                    Picker::indexed(["North", "South"])
                        .style(PickerStyle::Radio)
                        .selection(0)
                        .on_change(reject),
                    cx.watch(rejected, |count| {
                        Text::new(format!("Rejected: {count}")).id("rejected")
                    }),
                    Button::new("Select Express").id("select-last").on_click(move |cx| {
                        for store in [menu, segmented, radio] {
                            cx.update(store, |service| *service = Service::Express);
                        }
                    }),
                    echo(cx, menu, "menu-value"),
                    echo(cx, segmented, "segmented-value"),
                    echo(cx, radio, "radio-value"),
                    cx.watch(changes, |changes| {
                        let changes = changes.iter().map(|service| format!("{service:?}")).collect::<Vec<_>>();
                        Text::new(format!("Changes: {}", changes.join(","))).id("changes")
                    }),
                    // Both pickers share a Store that starts with no service.
                    Picker::new(unset_options()).bind(unset).id("unset-menu-picker"),
                    Picker::new(unset_options())
                        .style(PickerStyle::Segmented)
                        .bind(unset)
                        .id("unset-segmented-picker"),
                    Button::new("Clear Service").id("clear-unset").on_click(move |cx| {
                        cx.update(unset, |service| *service = None);
                    }),
                    cx.watch(unset, |service| {
                        Text::new(format!("Unset: {service:?}")).id("unset-value")
                    }),
                ])
                .spacing(10.0),
            )
    }
}
