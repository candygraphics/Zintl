//! Shows every desktop control, presentation, and menu in one app.
//!
//! The views also describe themselves to screen readers: labels name controls
//! without visible text, `accessibility_labeled_by` links fields to the text
//! beside them, and actions announce their result.

use zintl_ui_desktop::accessibility::*;
use zintl_ui_desktop::dialog::*;
use zintl_ui_desktop::menu::*;
use zintl_ui_desktop::*;

/// The sizes the size picker selects by value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ItemSize {
    Small,
    Medium,
    Large,
}

/// The pages the sidebar selects by key.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Page {
    Controls,
    Containers,
    Buttons,
    Dialogs,
}

impl ItemKey for Page {
    fn item_id(&self) -> String {
        format!("{self:?}").to_lowercase()
    }
}

#[derive(Default)]
struct Gallery {
    page: Store<Option<Page>>,
    query: Store<String>,
    status: Store<String>,
    notifications: Store<bool>,
    bold: Store<bool>,
    agree: Store<bool>,
    size: Store<ItemSize>,
    layout: Store<&'static str>,
    priority: Store<usize>,
    fruit: Store<String>,
    volume: Store<f64>,
    quantity: Store<f64>,
    tint: Store<Color>,
    search: Store<String>,
    tab: Store<&'static str>,
    scroll_status: Store<ScrollStatus>,
    info: Store<bool>,
    confirm_delete: Store<bool>,
    open_file: Store<bool>,
    save_file: Store<bool>,
    color_panel: Store<bool>,
    inspector: Store<bool>,
}

impl Gallery {
    const PAGES: [(Page, &'static str, &'static str); 4] = [
        (Page::Controls, "Controls", "slider.horizontal.3"),
        (Page::Containers, "Containers", "square.split.2x1"),
        (Page::Buttons, "Buttons & Menus", "cursorarrow.click"),
        (Page::Dialogs, "Dialogs & Windows", "macwindow.on.rectangle"),
    ];

    fn sidebar(&self, query: &str) -> Sidebar<Page> {
        let query = query.to_lowercase();
        let items = Self::PAGES
            .iter()
            .filter(|(_, title, _)| title.to_lowercase().contains(&query))
            .map(|(page, title, symbol)| SidebarItem::new(*page, *title).system_image(*symbol));
        Sidebar::new([SidebarSection::new(items).title("Gallery")])
            .bind(self.page)
            .searchable(self.query)
            .search_placeholder("Search")
    }

    fn controls(&self) -> VStack {
        let fruit = self.fruit;
        VStack::new(list![
            Toggle::new("Notifications").bind(self.notifications),
            Toggle::new("I agree to the terms")
                .style(ToggleStyle::Checkbox)
                .bind(self.agree),
            Toggle::new("Bold")
                .style(ToggleStyle::Button)
                .bind(self.bold),
            // Each field is named by the text beside it.
            HStack::new(list![
                Text::new("Size").id("size-label"),
                Picker::new([
                    (ItemSize::Small, "Small"),
                    (ItemSize::Medium, "Medium"),
                    (ItemSize::Large, "Large"),
                ])
                .bind(self.size)
                .accessibility_labeled_by("size-label"),
            ]),
            Picker::new([("list", "List"), ("grid", "Grid"), ("columns", "Columns")])
                .style(PickerStyle::Segmented)
                .bind(self.layout)
                .accessibility_label("Layout"),
            Picker::indexed(["Low", "Normal", "High"])
                .style(PickerStyle::Radio)
                .bind(self.priority)
                .accessibility_label("Priority"),
            ComboBox::new(["Apple", "Banana", "Cherry"])
                .placeholder("Fruit")
                .bind(fruit),
            HStack::new(list![
                Text::new("Volume").id("volume-label"),
                Slider::new(0.0..=100.0)
                    .ticks(11)
                    .bind(self.volume)
                    .accessibility_labeled_by("volume-label"),
            ]),
            HStack::new(list![
                Text::new("Quantity").id("quantity-label"),
                NumberField::new()
                    .range(0.0..=99.0)
                    .step(1.0)
                    .bind(self.quantity)
                    .accessibility_labeled_by("quantity-label"),
            ]),
            HStack::new(list![
                Text::new("Tint").id("tint-label"),
                ColorWell::new()
                    .bind(self.tint)
                    .accessibility_labeled_by("tint-label"),
            ]),
            SearchField::new()
                .placeholder("Search items")
                .bind(self.search),
            self.summary(),
        ])
        .context_menu([MenuItem::new("Duplicate").shortcut(Shortcut::command("d"))])
        .spacing(12.0)
        .padding(20.0)
    }

    fn containers(&self) -> VStack {
        let rows = (1..=40)
            .map(|row| ElementFactory::new(Text::new(format!("Row {row}"))))
            .collect::<ListFactory<RenderNode>>();
        VStack::new(list![
            TabView::new([
                Tab::new(
                    "general",
                    "General",
                    VStack::new(list![
                        Toggle::new("Show previews").bind(self.notifications),
                        Text::new("Tabs keep each page mounted."),
                    ]),
                ),
                Tab::new(
                    "advanced",
                    "Advanced",
                    VStack::new(list![Text::new("Advanced settings live here.")]),
                ),
            ])
            .bind(self.tab)
            .minimum_size(Size::new(320.0, 140.0)),
            Text::new("Scrollable list"),
            ScrollView::new(VStack::new(rows).spacing(6.0).padding(8.0))
                .bind_offset(self.scroll_status),
        ])
        .spacing(12.0)
        .padding(20.0)
        .grow()
    }

    fn buttons(&self) -> VStack {
        let status = self.status;
        // The status line changes away from the focused button, so it is also announced.
        let report = move |message: &'static str| {
            move |cx: &mut Context<'_>| {
                cx.update(status, |status| *status = message.into());
                cx.announce(message, AnnouncementPriority::Medium);
            }
        };
        VStack::new(list![
            // A labeled stack reads as one named group.
            HStack::new(list![
                Button::new("Save")
                    .role(ButtonRole::Default)
                    .on_click(report("Saved")),
                Button::new("Cancel")
                    .role(ButtonRole::Cancel)
                    .on_click(report("Cancelled")),
                Button::new("Delete")
                    .role(ButtonRole::Destructive)
                    .on_click(report("Deleted"))
                    .accessibility_hint("Deletes the item without asking"),
            ])
            .accessibility_label("Document actions"),
            HStack::new(list![
                Button::new("Share")
                    .system_image("square.and.arrow.up")
                    .on_click(report("Shared")),
                // Buttons without a title need a label.
                Button::icon("gearshape", "Settings")
                    .style(ButtonStyle::Borderless)
                    .minimum_size(Size::new(32.0, 32.0))
                    .on_click(report("Gear")),
                Button::help()
                    .minimum_size(Size::new(32.0, 32.0))
                    .on_click(report("Help"))
                    .accessibility_label("Help"),
                Button::new("Toolbar")
                    .style(ButtonStyle::Toolbar)
                    .on_click(report("Toolbar")),
            ]),
            Button::new("Show Info")
                .on_click({
                    let info = self.info;
                    move |cx| cx.update(info, |shown| *shown = true)
                })
                .popover(
                    Popover::new(
                        VStack::new(list![
                            Text::new("Popover"),
                            Toggle::new("Notifications").bind(self.notifications),
                        ])
                        .spacing(8.0),
                    )
                    .bind_presented(self.info),
                ),
            Text::new("Right-click for a context menu")
                .minimum_size(Size::new(240.0, 20.0))
                .accessibility_hint("Open its menu with Control-Option-Shift-M")
                .context_menu([
                    MenuItem::new("Rename").on_select(report("Rename chosen")),
                    MenuItem::new("Duplicate")
                        .shortcut(Shortcut::command("d"))
                        .on_select(report("Duplicate chosen")),
                    MenuItem::separator(),
                    MenuItem::standard(StandardAction::Copy),
                ]),
            Status(self.status),
        ])
        .spacing(16.0)
        .padding(20.0)
    }

    fn dialogs(&self) -> VStack {
        let show = |store: Store<bool>| move |cx: &mut Context<'_>| cx.update(store, |v| *v = true);
        VStack::new(list![
            Button::new("Delete…")
                .minimum_size(Size::new(160.0, 32.0))
                .on_click(show(self.confirm_delete)),
            Button::new("Open…")
                .minimum_size(Size::new(160.0, 32.0))
                .on_click(show(self.open_file)),
            Button::new("Save As…")
                .minimum_size(Size::new(160.0, 32.0))
                .on_click(show(self.save_file)),
            Button::new("Colors…")
                .minimum_size(Size::new(160.0, 32.0))
                .on_click(show(self.color_panel)),
            Button::new("Inspector")
                .minimum_size(Size::new(160.0, 32.0))
                .on_click(show(self.inspector)),
            Status(self.status),
        ])
        .spacing(12.0)
        .padding(20.0)
    }

    fn summary(&self) -> Summary {
        Summary {
            volume: self.volume,
            quantity: self.quantity,
            fruit: self.fruit,
        }
    }

    fn main_menu(&self) -> MainMenu {
        let status = self.status;
        let inspector = self.inspector;
        MainMenu::standard()
            .app_name("Zintl Gallery")
            .settings(move |cx| cx.update(status, |status| *status = "Settings".into()))
            .extend(
                StandardMenu::File,
                [MenuItem::new("Show Inspector")
                    .shortcut(Shortcut::command("i"))
                    .on_select(move |cx| cx.update(inspector, |open| *open = true))],
            )
            .menu(Menu::new(
                "Gallery",
                [
                    MenuItem::new("Say Hello")
                        .shortcut(Shortcut::command("h").shift())
                        .on_select(move |cx| cx.update(status, |status| *status = "Hello".into())),
                    MenuItem::new("Disabled Item").enabled(false),
                ],
            ))
    }
}

/// Shows values chosen by the controls.
#[derive(Clone)]
struct Summary {
    volume: Store<f64>,
    quantity: Store<f64>,
    fruit: Store<String>,
}

impl View for Summary {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Text::new(format!(
            "Volume {:.0}, quantity {:.0}, fruit {:?}",
            cx.get(self.volume),
            cx.get(self.quantity),
            cx.get(self.fruit),
        ))
        .minimum_size(Size::new(320.0, 20.0))
    }
}

/// Shows the last action reported by a button, menu, or dialog.
#[derive(Clone)]
struct Status(Store<String>);

impl View for Status {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Text::new(format!("Status: {}", cx.get(self.0))).minimum_size(Size::new(320.0, 20.0))
    }
}

impl View for Gallery {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.page = cx.store(Some(Page::Controls));
        self.query = cx.store(String::new());
        self.status = cx.store("Ready".into());
        self.notifications = cx.store(true);
        self.bold = cx.store(false);
        self.agree = cx.store(false);
        self.size = cx.store(ItemSize::Medium);
        self.layout = cx.store("list");
        self.priority = cx.store(1);
        self.fruit = cx.store(String::new());
        self.volume = cx.store(40.0);
        self.quantity = cx.store(3.0);
        self.tint = cx.store(Color::rgb(0.0, 0.48, 1.0));
        self.search = cx.store(String::new());
        self.tab = cx.store("general");
        self.scroll_status = cx.store(ScrollStatus::default());
        self.info = cx.store(false);
        self.confirm_delete = cx.store(false);
        self.open_file = cx.store(false);
        self.save_file = cx.store(false);
        self.color_panel = cx.store(false);
        self.inspector = cx.store(false);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let status = self.status;
        let report = move |cx: &mut Context<'_>, message: String| {
            cx.update(status, |status| *status = message);
        };
        let content = match cx.get(self.page).unwrap_or(Page::Controls) {
            Page::Controls => self.controls(),
            Page::Containers => self.containers(),
            Page::Buttons => self.buttons(),
            Page::Dialogs => self.dialogs(),
        };
        let window = Window::new("Zintl Gallery")
            .size(Size::new(820.0, 620.0))
            .sidebar(self.sidebar(cx.get(self.query)))
            .content(content.fill_width())
            .alert(
                Alert::new("Delete the selected item?")
                    .message("This action cannot be undone.")
                    .button(
                        AlertButton::new("Delete")
                            .role(ButtonRole::Destructive)
                            .on_click(move |cx| report(cx, "Item deleted".into())),
                    )
                    .button(AlertButton::new("Cancel").role(ButtonRole::Cancel))
                    .bind_presented(self.confirm_delete),
            )
            .file_dialog(
                FileDialog::open()
                    .multiple(true)
                    .bind_presented(self.open_file)
                    .on_select(move |cx, paths| {
                        report(cx, format!("Opened {} item(s)", paths.len()))
                    })
                    .on_cancel(move |cx| report(cx, "Open cancelled".into())),
            )
            .file_dialog(
                FileDialog::save()
                    .file_name("Untitled.txt")
                    .allowed_extensions(["txt"])
                    .bind_presented(self.save_file)
                    .on_select(move |cx, paths| {
                        report(cx, format!("Save to {}", paths[0].display()))
                    }),
            )
            .color_panel(
                ColorPanel::new()
                    .bind(self.tint)
                    .bind_presented(self.color_panel),
            );
        let inspector = Window::panel("Inspector")
            .size(Size::new(260.0, 200.0))
            .position(Point::new(960.0, 160.0))
            .content(
                VStack::new(list![
                    Text::new("Tint"),
                    ColorWell::new().bind(self.tint),
                    Toggle::new("Notifications").bind(self.notifications),
                ])
                .spacing(8.0)
                .padding(16.0),
            )
            .bind_open(self.inspector);
        list![self.main_menu(), window, inspector]
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> Result<(), AppError> {
    App::new(Gallery::default()).run()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
