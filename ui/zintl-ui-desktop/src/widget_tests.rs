//! Contracts for the desktop controls, presentations, and menus.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use super::*;
use zintl_ui_appkit::{ButtonType, MenuItemState, MenuRole, WindowStyle};

/// Creates Stores in `init`, publishes them, and renders from them.
struct Harness<S: Copy + 'static, E> {
    create: fn(&mut Context<'_>) -> S,
    render: Box<dyn Fn(S) -> E>,
    stores: Rc<Cell<Option<S>>>,
}

impl<S: Copy + 'static, E: IntoElement<Output = RenderNode> + 'static> View for Harness<S, E> {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.stores.set(Some((self.create)(cx)));
    }

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        (self.render)(self.stores.get().expect("init creates the Stores"))
    }
}

impl<S: Copy + 'static, E: IntoElement<Output = RenderNode> + 'static> Harness<S, E> {
    fn mount(create: fn(&mut Context<'_>) -> S, render: impl Fn(S) -> E + 'static) -> (App, S) {
        let stores = Rc::new(Cell::new(None));
        let app = App::new(Self {
            create,
            render: Box::new(render),
            stores: stores.clone(),
        });
        let stores = stores.get().expect("mounting runs init");
        (app, stores)
    }
}

impl App {
    fn get<T: Clone + 'static>(&mut self, store: Store<T>) -> T {
        self.composer.context(|cx| cx.get(store).clone())
    }

    fn set<T: 'static>(&mut self, store: Store<T>, value: T) {
        self.composer
            .context(|cx| cx.update(store, |current| *current = value));
        self.composer.flush();
    }
}

/// Records how many times an action ran.
#[derive(Clone, Default)]
struct Calls(Rc<Cell<usize>>);

impl Calls {
    fn action(&self) -> impl for<'a> Fn(&mut Context<'a>) + 'static {
        let calls = self.0.clone();
        move |_| calls.set(calls.get() + 1)
    }

    fn count(&self) -> usize {
        self.0.get()
    }
}

#[test]
fn switch_toggle_round_trips_through_its_store() {
    // Verifies an unlabeled Toggle renders an NSSwitch that writes user changes to its Store.
    let (mut app, on) = Harness::mount(|cx| cx.store(true), |on| Toggle::new("").bind(on));
    assert!(matches!(
        app.render(),
        RenderNode::NSSwitch { on: true, .. }
    ));

    assert!(app.dispatch_at(&[0], Event::StateChanged { on: false }));

    assert!(!app.get(on));
    assert!(matches!(
        app.render(),
        RenderNode::NSSwitch { on: false, .. }
    ));
}

#[test]
fn toggle_styles_use_native_two_state_controls() {
    // Verifies switch rows lead with the label and other styles use NSButton types.
    let row = App::new(Toggle::new("Wi-Fi")).render_tree();
    assert!(matches!(row.value, RenderNode::NSView { .. }));
    assert!(matches!(
        &row.children[0].value,
        RenderNode::NSTextField { value, .. } if value == "Wi-Fi"
    ));
    assert!(matches!(row.children[1].value, RenderNode::NSSwitch { .. }));

    let checkbox = App::new(Toggle::new("Agree").style(ToggleStyle::Checkbox)).render();
    let button = App::new(Toggle::new("Bold").style(ToggleStyle::Button)).render();
    assert!(matches!(
        checkbox,
        RenderNode::NSButton {
            button_type: ButtonType::Checkbox,
            ..
        }
    ));
    assert!(matches!(
        button,
        RenderNode::NSButton {
            button_type: ButtonType::PushOnPushOff,
            ..
        }
    ));
}

#[test]
fn radio_picker_selects_exactly_one_option() {
    // Verifies radio buttons reflect the Store and a newly checked radio selects its index.
    let (mut app, selected) = Harness::mount(
        |cx| cx.store(0_usize),
        |selected| {
            Picker::new(["Low", "Normal", "High"])
                .style(PickerStyle::Radio)
                .bind(selected)
        },
    );
    let states = |app: &App| {
        app.render_tree()
            .children
            .iter()
            .map(|radio| match radio.value {
                RenderNode::NSButton {
                    button_type: ButtonType::Radio,
                    on,
                    ..
                } => on,
                _ => panic!("radio pickers render radio buttons"),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(states(&app), [true, false, false]);

    assert!(app.dispatch_at(&[0, 2], Event::StateChanged { on: true }));

    assert_eq!(app.get(selected), 2);
    assert_eq!(states(&app), [false, false, true]);
}

#[test]
fn menu_and_segmented_pickers_render_bound_selection() {
    // Verifies other picker styles map to pop-up buttons and segmented controls.
    let (mut app, selected) = Harness::mount(
        |cx| cx.store(1_usize),
        |selected| Picker::new(["A", "B"]).bind(selected),
    );
    assert!(matches!(
        app.render(),
        RenderNode::NSPopUpButton {
            selected: Some(1),
            ..
        }
    ));
    assert!(app.dispatch_at(&[0], Event::SelectionChanged { index: 0 }));
    assert_eq!(app.get(selected), 0);

    let segmented = App::new(Picker::new(["A", "B"]).style(PickerStyle::Segmented)).render();
    assert!(matches!(
        segmented,
        RenderNode::NSSegmentedControl { segments, .. } if segments == ["A", "B"]
    ));
}

#[test]
fn number_field_clamps_entered_values() {
    // Verifies values outside the declared range are clamped before reaching the Store.
    let (mut app, value) = Harness::mount(
        |cx| cx.store(5.0_f64),
        |value| NumberField::new().range(0.0..=10.0).bind(value),
    );

    assert!(app.dispatch_at(&[0], Event::ValueChanged { value: 42.0 }));

    assert_eq!(app.get(value), 10.0);
    assert!(matches!(
        app.render(),
        RenderNode::NSNumberField {
            value: 10.0,
            minimum: 0.0,
            maximum: 10.0,
            ..
        }
    ));
}

#[test]
fn slider_combo_box_search_and_color_write_their_stores() {
    // Verifies each value control forwards its native event payload to its Store.
    #[derive(Clone, Copy)]
    struct Stores {
        volume: Store<f64>,
        fruit: Store<String>,
        query: Store<String>,
        tint: Store<Color>,
    }
    let (mut app, stores) = Harness::mount(
        |cx| Stores {
            volume: cx.store(0.0),
            fruit: cx.store(String::new()),
            query: cx.store(String::new()),
            tint: cx.store(Color::BLACK),
        },
        |stores| {
            VStack::new(list![
                Slider::new(0.0..=100.0).bind(stores.volume),
                ComboBox::new(["Apple"]).bind(stores.fruit),
                SearchField::new().bind(stores.query),
                ColorWell::new().bind(stores.tint),
            ])
        },
    );

    app.dispatch_at(&[0, 0], Event::ValueChanged { value: 30.0 });
    app.dispatch_at(
        &[0, 1],
        Event::TextChanged {
            value: "Apple".into(),
        },
    );
    app.dispatch_at(&[0, 2], Event::TextChanged { value: "ap".into() });
    app.dispatch_at(
        &[0, 3],
        Event::ColorChanged {
            color: Color::WHITE,
        },
    );

    assert_eq!(app.get(stores.volume), 30.0);
    assert_eq!(app.get(stores.fruit), "Apple");
    assert_eq!(app.get(stores.query), "ap");
    assert_eq!(app.get(stores.tint), Color::WHITE);
}

#[test]
fn button_roles_and_styles_configure_nsbutton() {
    // Verifies roles become key equivalents or destructive styling and styles pick a bezel.
    let render = |button: Button| App::new(button).render();
    assert!(matches!(
        render(Button::new("OK").role(ButtonRole::Default)),
        RenderNode::NSButton { key_equivalent: Some(key), .. } if key == "\r"
    ));
    assert!(matches!(
        render(Button::new("Cancel").role(ButtonRole::Cancel)),
        RenderNode::NSButton { key_equivalent: Some(key), .. } if key == "\u{1b}"
    ));
    assert!(matches!(
        render(Button::new("Delete").role(ButtonRole::Destructive)),
        RenderNode::NSButton {
            destructive: true,
            ..
        }
    ));
    assert!(matches!(
        render(
            Button::new("")
                .style(ButtonStyle::Borderless)
                .system_image("gearshape")
        ),
        RenderNode::NSButton { bezel: None, system_image: Some(name), .. } if name == "gearshape"
    ));
}

#[test]
fn context_menu_routes_custom_items_by_position() {
    // Verifies context menus attach to their view, assign IDs, and keep actionless items disabled.
    let calls = Calls::default();
    let mut app = App::new(Button::new("Item").context_menu([
        MenuItem::new("Rename").on_select(calls.action()),
        MenuItem::separator(),
        MenuItem::standard(StandardAction::Copy),
        MenuItem::new("Unavailable"),
    ]));
    let tree = app.render_tree();
    let RenderNode::NSMenu { menu } = &tree.children[0].value else {
        panic!("context menus render as a child NSMenu");
    };
    assert!(matches!(
        &menu.items[0],
        MenuItemState::Action { id: Some(id), enabled: true, .. } if id == "context.0"
    ));
    assert_eq!(menu.items[1], MenuItemState::Separator);
    assert!(matches!(
        &menu.items[2],
        MenuItemState::Action {
            id: None,
            standard: Some(StandardAction::Copy),
            shortcut: Some(_),
            ..
        }
    ));
    assert!(matches!(
        &menu.items[3],
        MenuItemState::Action { enabled: false, .. }
    ));

    assert!(app.dispatch_at(
        &[0, 0],
        Event::MenuItemSelected {
            id: "context.0".into(),
        }
    ));
    assert_eq!(calls.count(), 1);
}

#[test]
fn popover_follows_its_presented_store() {
    // Verifies popovers render content as children and AppKit dismissal clears the Store.
    let (mut app, shown) = Harness::mount(
        |cx| cx.store(false),
        |shown| Button::new("Info").popover(Popover::new(Text::new("Details")).bind(shown)),
    );
    let popover = |app: &App| app.render_tree().children[0].clone();
    assert!(matches!(
        popover(&app).value,
        RenderNode::NSPopover {
            presented: false,
            ..
        }
    ));
    assert_eq!(popover(&app).children.len(), 1);

    app.set(shown, true);
    assert!(matches!(
        popover(&app).value,
        RenderNode::NSPopover {
            presented: true,
            ..
        }
    ));

    assert!(app.dispatch_at(&[0, 0], Event::PopoverClosed));
    assert!(!app.get(shown));
}

#[test]
fn alert_buttons_run_their_action_and_dismiss() {
    // Verifies a chosen alert button runs once, while a programmatic dismissal runs nothing.
    let deletes = Calls::default();
    let action = deletes.clone();
    let (mut app, shown) = Harness::mount(
        |cx| cx.store(true),
        move |shown| {
            Window::new(Rect::new(0.0, 0.0, 400.0, 300.0), "Alerts").alert(
                Alert::new("Delete?")
                    .button(
                        AlertButton::new("Delete")
                            .role(ButtonRole::Destructive)
                            .on_click(action.action()),
                    )
                    .button(AlertButton::new("Cancel").role(ButtonRole::Cancel))
                    .bind(shown),
            )
        },
    );
    let tree = app.render_tree();
    assert!(matches!(
        &tree.children[0].value,
        RenderNode::NSAlert { presented: true, alert }
            if alert.buttons == [
                ("Delete".to_owned(), ButtonRole::Destructive),
                ("Cancel".to_owned(), ButtonRole::Cancel),
            ]
    ));

    assert!(app.dispatch_at(&[0, 0], Event::AlertResponse { button: Some(0) }));
    assert_eq!(deletes.count(), 1);
    assert!(!app.get(shown));

    app.set(shown, true);
    app.dispatch_at(&[0, 0], Event::AlertResponse { button: None });
    assert_eq!(deletes.count(), 1);
    assert!(!app.get(shown));
}

#[test]
fn file_dialog_reports_paths_or_cancellation() {
    // Verifies save panels report the chosen path, cancellation, and clear their Store.
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let cancels = Calls::default();
    let (received, cancel) = (chosen.clone(), cancels.clone());
    let (mut app, shown) = Harness::mount(
        |cx| cx.store(true),
        move |shown| {
            let received = received.clone();
            Window::new(Rect::new(0.0, 0.0, 400.0, 300.0), "Files").file_dialog(
                FileDialog::save()
                    .file_name("notes.txt")
                    .bind(shown)
                    .on_select(move |_, paths| received.borrow_mut().extend_from_slice(paths))
                    .on_cancel(cancel.action()),
            )
        },
    );
    assert!(matches!(
        &app.render_tree().children[0].value,
        RenderNode::NSSavePanel { presented: true, panel }
            if panel.file_name.as_deref() == Some("notes.txt")
    ));

    app.dispatch_at(
        &[0, 0],
        Event::FileDialogCompleted {
            paths: Some(vec![PathBuf::from("/tmp/notes.txt")]),
        },
    );
    assert_eq!(*chosen.borrow(), [PathBuf::from("/tmp/notes.txt")]);
    assert!(!app.get(shown));

    app.set(shown, true);
    app.dispatch_at(&[0, 0], Event::FileDialogCompleted { paths: None });
    assert_eq!(cancels.count(), 1);
}

#[test]
fn color_panel_writes_color_and_presentation_stores() {
    // Verifies the color panel edits its bound color and closing it clears the presentation.
    let (mut app, (color, shown)) = Harness::mount(
        |cx| (cx.store(Color::BLACK), cx.store(true)),
        |(color, shown)| {
            Window::new(Rect::new(0.0, 0.0, 400.0, 300.0), "Colors")
                .color_panel(ColorPanel::new().bind_color(color).bind(shown))
        },
    );
    let red = Color::rgb(1.0, 0.0, 0.0);

    app.dispatch_at(&[0, 0], Event::ColorChanged { color: red });
    assert_eq!(app.get(color), red);
    assert!(matches!(
        app.render_tree().children[0].value,
        RenderNode::NSColorPanel { color, presented: true, .. } if color == red
    ));

    app.dispatch_at(&[0, 0], Event::ColorPanelClosed);
    assert!(!app.get(shown));
}

#[test]
fn bound_window_closes_and_reopens_from_its_store() {
    // Verifies closing a bound panel removes it and setting the Store declares it again.
    let (mut app, open) = Harness::mount(
        |cx| cx.store(true),
        |open| Window::panel(Rect::new(0.0, 0.0, 200.0, 200.0), "Inspector").bind_open(open),
    );
    assert!(matches!(
        app.render(),
        RenderNode::NSWindow {
            style: WindowStyle::Panel,
            ..
        }
    ));

    assert!(app.dispatch_at(&[0], Event::WindowDidClose));
    assert!(!app.get(open));
    assert!(app.render_roots().is_empty());

    app.set(open, true);
    assert_eq!(app.render_roots().len(), 1);
}

#[test]
fn main_menu_orders_standard_and_custom_menus() {
    // Verifies the macOS menu order, standard titles, extensions, and custom item routing.
    let runs = Calls::default();
    let run = runs.clone();
    let (mut app, ()) = Harness::mount(
        |_| (),
        move |()| {
            list![
                MainMenu::standard()
                    .app_name("Demo")
                    .extend(StandardMenu::File, [MenuItem::new("New")])
                    .menu(Menu::new(
                        "Tools",
                        [MenuItem::new("Run").on_select(run.action())]
                    )),
                Window::new(Rect::new(0.0, 0.0, 200.0, 200.0), "Demo"),
            ]
        },
    );
    let roots = app.render_roots();
    let RenderNode::NSMainMenu { menus } = &roots[0].value else {
        panic!("the main menu renders as a root node");
    };
    let titles = menus
        .iter()
        .map(|menu| menu.title.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        titles,
        ["", "File", "Edit", "View", "Tools", "Window", "Help"]
    );
    let title = |item: &MenuItemState| match item {
        MenuItemState::Action { title, .. } => title.clone(),
        MenuItemState::Submenu(menu) => menu.title.clone(),
        MenuItemState::Separator => String::new(),
    };
    assert_eq!(title(&menus[0].items[0]), "About Demo");
    assert_eq!(title(menus[0].items.last().unwrap()), "Quit Demo");
    assert!(matches!(
        &menus[0].items[2],
        MenuItemState::Submenu(services) if services.role == Some(MenuRole::Services)
    ));
    assert_eq!(title(&menus[1].items[0]), "New");
    assert_eq!(menus[5].role, Some(MenuRole::Windows));
    assert_eq!(menus[6].role, Some(MenuRole::Help));

    assert!(app.dispatch_at(&[0], Event::MenuItemSelected { id: "m4.0".into() }));
    assert_eq!(runs.count(), 1);
}

#[test]
fn menu_localizers_translate_standard_text() {
    // Verifies standard titles come from the localizer and catalog keys stay unique.
    let roots = App::new(list![
        MainMenu::standard()
            .app_name("Demo")
            .localizer(JapaneseMenuLocalizer)
            .settings(|_| {}),
        Window::new(Rect::new(0.0, 0.0, 200.0, 200.0), "Demo"),
    ])
    .render_roots();
    let RenderNode::NSMainMenu { menus } = &roots[0].value else {
        panic!("the main menu renders as a root node");
    };
    assert!(matches!(
        &menus[0].items[0],
        MenuItemState::Action { title, .. } if title == "Demoについて"
    ));
    assert!(matches!(
        &menus[0].items[2],
        MenuItemState::Action { title, shortcut: Some(_), .. } if title == "設定…"
    ));
    assert_eq!(menus[2].title, "編集");

    let texts = [
        StandardText::About,
        StandardText::Settings,
        StandardText::Copy,
        StandardText::Paste,
        StandardText::WindowMenu,
        StandardText::Help,
    ];
    let keys = texts
        .iter()
        .map(|text| text.key())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(keys.len(), texts.len());
}

#[test]
fn sidebar_search_updates_its_query_store() {
    // Verifies a searchable sidebar renders its query and writes native edits back.
    let (mut app, query) = Harness::mount(
        |cx| cx.store(String::new()),
        |query| {
            Window::new(Rect::new(0.0, 0.0, 400.0, 300.0), "Search").sidebar(
                Sidebar::new([SidebarSection::new([SidebarItem::new("home", "Home")])])
                    .searchable(query)
                    .search_placeholder("Search"),
            )
        },
    );

    assert!(app.dispatch_at(&[0], Event::SidebarSearchChanged { value: "ho".into() }));

    assert_eq!(app.get(query), "ho");
    assert!(matches!(
        app.render(),
        RenderNode::NSWindow { sidebar: Some(SidebarState { search: Some(search), .. }), .. }
            if search.value == "ho" && search.placeholder.as_deref() == Some("Search")
    ));
}

#[test]
fn tab_view_hosts_one_page_per_tab() {
    // Verifies each tab renders a page container and native selection writes the Store.
    let (mut app, selected) = Harness::mount(
        |cx| cx.store(0_usize),
        |selected| {
            TabView::new([
                Tab::new("General", Text::new("General page")),
                Tab::new("Advanced", Text::new("Advanced page")),
            ])
            .bind(selected)
        },
    );
    let tree = app.render_tree();
    assert!(matches!(
        &tree.value,
        RenderNode::NSTabView { titles, selected: 0, .. } if titles == &["General", "Advanced"]
    ));
    assert_eq!(tree.children.len(), 2);
    assert!(
        tree.children
            .iter()
            .all(|page| matches!(page.value, RenderNode::NSView { .. }) && page.children.len() == 1)
    );

    assert!(app.dispatch_at(&[0], Event::SelectionChanged { index: 1 }));
    assert_eq!(app.get(selected), 1);
}

#[test]
fn scroll_view_fills_its_parent_and_hosts_content() {
    // Verifies scroll views grow into free space and keep their content as a child.
    let tree = App::new(ScrollView::new(VStack::new(list![Text::new("Row")]))).render_tree();
    assert!(matches!(
        tree.value,
        RenderNode::NSScrollView {
            layout: LayoutStyle {
                width: LayoutDimension::Percent(1.0),
                grow: 1.0,
                ..
            },
            ..
        }
    ));
    assert_eq!(tree.children.len(), 1);
}

#[test]
fn scroll_view_binding_saves_and_restores_offset() {
    // Verifies native scrolling updates the bound status, which the node carries for restoring.
    let (mut app, status) = Harness::mount(
        |cx| cx.store(ScrollStatus::default()),
        |status| ScrollView::new(VStack::new(list![Text::new("Row")])).bind(status),
    );
    assert!(app.dispatch_at(&[0], Event::Scrolled { x: 0.0, y: 120.0 }));
    assert_eq!(app.get(status), ScrollStatus { x: 0.0, y: 120.0 });
    assert!(matches!(
        app.render_tree().value,
        RenderNode::NSScrollView {
            status: Some(ScrollStatus { x: 0.0, y: 120.0 }),
            ..
        }
    ));
}
