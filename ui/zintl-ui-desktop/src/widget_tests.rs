//! Contracts for the desktop controls, presentations, and menus.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use super::*;
use zintl_ui_backend_core::{
    AlertButton as AlertButtonDescription, AlertButtonRole, CommandItem, CommandRole,
    FileDialogMode, MenuRole as CommandMenuRole, NumberFormat, Sidebar as SidebarDescription,
};
use zintl_ui_native::{ButtonType, WindowStyle};

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
    // Verifies an unlabeled Toggle renders a switch that writes user changes to its Store.
    let (mut app, on) = Harness::mount(|cx| cx.store(true), |on| Toggle::new("").bind(on));
    assert!(matches!(
        app.render(),
        RenderNode::View {
            kind: ViewKind::Switch { on: true, .. },
            ..
        }
    ));

    assert!(app.dispatch_at(&[0], Event::StateChanged { on: false }));

    assert!(!app.get(on));
    assert!(matches!(
        app.render(),
        RenderNode::View {
            kind: ViewKind::Switch { on: false, .. },
            ..
        }
    ));
}

#[test]
fn toggle_styles_use_native_two_state_controls() {
    // Verifies switch rows lead with the label and other styles use button types.
    let row = App::new(Toggle::new("Wi-Fi")).render_tree();
    assert!(matches!(
        row.value,
        RenderNode::View {
            kind: ViewKind::Container,
            ..
        }
    ));
    assert!(matches!(
        &row.children[0].value,
        RenderNode::View { kind: ViewKind::TextField { value, .. }, .. } if value == "Wi-Fi"
    ));
    assert!(matches!(
        row.children[1].value,
        RenderNode::View {
            kind: ViewKind::Switch { .. },
            ..
        }
    ));

    let checkbox = App::new(Toggle::new("Agree").style(ToggleStyle::Checkbox)).render();
    let button = App::new(Toggle::new("Bold").style(ToggleStyle::Button)).render();
    assert!(matches!(
        checkbox,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                button_type: ButtonType::Checkbox,
                ..
            }),
            ..
        }
    ));
    assert!(matches!(
        button,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                button_type: ButtonType::PushOnPushOff,
                ..
            }),
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
                RenderNode::View {
                    kind:
                        ViewKind::Button(zintl_ui_backend_core::Button {
                            button_type: ButtonType::Radio,
                            on,
                            ..
                        }),
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
        RenderNode::View {
            kind: ViewKind::PopUpButton {
                selected: Some(1),
                ..
            },
            ..
        }
    ));
    assert!(app.dispatch_at(&[0], Event::SelectionChanged { index: 0 }));
    assert_eq!(app.get(selected), 0);

    let segmented = App::new(Picker::new(["A", "B"]).style(PickerStyle::Segmented)).render();
    assert!(matches!(
        segmented,
        RenderNode::View { kind: ViewKind::SegmentedControl { segments, .. }, .. } if segments == ["A", "B"]
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
        RenderNode::View {
            kind: ViewKind::NumberField {
                value: 10.0,
                format: NumberFormat {
                    minimum: 0.0,
                    maximum: 10.0,
                    ..
                },
            },
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
        RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { key_equivalent: Some(key), .. }), .. } if key == "\r"
    ));
    assert!(matches!(
        render(Button::new("Cancel").role(ButtonRole::Cancel)),
        RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { key_equivalent: Some(key), .. }), .. } if key == "\u{1b}"
    ));
    assert!(matches!(
        render(Button::new("Delete").role(ButtonRole::Destructive)),
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                destructive: true,
                ..
            }),
            ..
        }
    ));
    assert!(matches!(
        render(
            Button::new("")
                .style(ButtonStyle::Borderless)
                .system_image("gearshape")
        ),
        RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { bezel: None, system_image: Some(name), .. }), .. } if name == "gearshape"
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
    let RenderNode::ContextMenu { items } = &tree.children[0].value else {
        panic!("context menus render as a child context menu");
    };
    assert!(matches!(
        &items[0],
        CommandItem { id: Some(id), enabled: true, .. } if id == "context.0"
    ));
    assert_eq!(items[1].role, Some(CommandRole::Separator));
    assert!(matches!(
        &items[2],
        CommandItem {
            id: None,
            role: Some(CommandRole::Copy),
            key: Some(_),
            ..
        }
    ));
    assert!(matches!(&items[3], CommandItem { enabled: false, .. }));

    assert!(app.dispatch_at(
        &[0, 0],
        Event::MenuItemSelected {
            id: "context.0".into(),
        }
    ));
    assert_eq!(calls.count(), 1);
}

#[test]
fn view_builders_chain_after_attachments() {
    // Verifies attachment modifiers keep the view type, so its own builders still apply.
    let app = App::new(
        VStack::new(list![Text::new("Row")])
            .context_menu([MenuItem::new("Duplicate")])
            .spacing(12.0)
            .padding(20.0),
    );
    let tree = app.render_tree();
    let RenderNode::View {
        kind: ViewKind::Container,
        layout,
        ..
    } = &tree.value
    else {
        panic!("stacks render as a container view");
    };
    assert_eq!((layout.gap, layout.padding), (12.0, 20.0));
    assert!(matches!(
        tree.children.last().map(|child| &child.value),
        Some(RenderNode::ContextMenu { .. })
    ));
}

#[test]
fn popover_follows_its_presented_store() {
    // Verifies popovers render content as children and native dismissal clears the Store.
    let (mut app, shown) = Harness::mount(
        |cx| cx.store(false),
        |shown| Button::new("Info").popover(Popover::new(Text::new("Details")).bind(shown)),
    );
    let popover = |app: &App| app.render_tree().children[0].clone();
    assert!(matches!(
        popover(&app).value,
        RenderNode::Popover {
            presented: false,
            ..
        }
    ));
    assert_eq!(popover(&app).children.len(), 1);

    app.set(shown, true);
    assert!(matches!(
        popover(&app).value,
        RenderNode::Popover {
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
        RenderNode::Alert { presented: true, config }
            if config.buttons == [
                AlertButtonDescription {
                    title: "Delete".into(),
                    role: AlertButtonRole::Destructive,
                },
                AlertButtonDescription {
                    title: "Cancel".into(),
                    role: AlertButtonRole::Cancel,
                },
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
        RenderNode::FileDialog { presented: true, config }
            if config.mode == FileDialogMode::Save
                && config.file_name.as_deref() == Some("notes.txt")
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
        RenderNode::ColorPanel { color, presented: true, .. } if color == red
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
        RenderNode::Window {
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
    let RenderNode::MainMenu(commands) = &roots[0].value else {
        panic!("the main menu renders as a root node");
    };
    let titles = commands
        .menus
        .iter()
        .map(|menu| menu.title.as_str())
        .collect::<Vec<_>>();
    assert_eq!(titles, ["File", "Edit", "View", "Tools", "Window", "Help"]);
    let app_items = &commands.app_menu.as_ref().unwrap().items;
    assert_eq!(app_items[0].title, "About Demo");
    assert_eq!(app_items.last().unwrap().title, "Quit Demo");
    assert!(matches!(
        &app_items[2].submenu,
        Some(services) if services.role == Some(CommandMenuRole::Services)
    ));
    assert_eq!(commands.menus[0].items[0].title, "New");
    assert_eq!(commands.menus[4].role, Some(CommandMenuRole::Windows));
    assert_eq!(commands.menus[5].role, Some(CommandMenuRole::Help));

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
    let RenderNode::MainMenu(commands) = &roots[0].value else {
        panic!("the main menu renders as a root node");
    };
    let app_items = &commands.app_menu.as_ref().unwrap().items;
    assert_eq!(app_items[0].title, "Demoについて");
    assert!(matches!(
        &app_items[2],
        CommandItem { title, key: Some(_), .. } if title == "設定…"
    ));
    assert_eq!(commands.menus[1].title, "編集");

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
fn standard_edit_menu_ends_with_a_find_submenu() {
    // Verifies the Edit menu offers text finder actions with macOS shortcuts and translated titles.
    let roots = App::new(list![
        MainMenu::standard().localizer(JapaneseMenuLocalizer),
        Window::new(Rect::new(0.0, 0.0, 200.0, 200.0), "Demo"),
    ])
    .render_roots();
    let RenderNode::MainMenu(commands) = &roots[0].value else {
        panic!("the main menu renders as a root node");
    };
    let find = commands.menus[1]
        .items
        .last()
        .and_then(|item| item.submenu.as_ref())
        .expect("the Edit menu ends with the Find submenu");
    let items = find
        .items
        .iter()
        .map(|item| (item.title.as_str(), item.role, item.key.as_deref()))
        .collect::<Vec<_>>();
    assert_eq!(
        items,
        [
            ("検索…", Some(CommandRole::Find), Some("f")),
            ("検索と置換…", Some(CommandRole::FindAndReplace), Some("f")),
            ("次を検索", Some(CommandRole::FindNext), Some("g")),
            ("前を検索", Some(CommandRole::FindPrevious), Some("g")),
            (
                "選択部分を検索に使用",
                Some(CommandRole::UseSelectionForFind),
                Some("e")
            ),
        ]
    );
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
        RenderNode::Window { sidebar: Some(SidebarDescription { search: Some(search), .. }), .. }
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
        RenderNode::View { kind: ViewKind::TabView { titles, selected: 0, .. }, .. } if titles == &["General", "Advanced"]
    ));
    assert_eq!(tree.children.len(), 2);
    assert!(tree.children.iter().all(|page| matches!(
        page.value,
        RenderNode::View {
            kind: ViewKind::Container,
            ..
        }
    ) && page.children.len() == 1));

    assert!(app.dispatch_at(&[0], Event::SelectionChanged { index: 1 }));
    assert_eq!(app.get(selected), 1);
}

#[test]
fn scroll_view_fills_its_parent_and_hosts_content() {
    // Verifies scroll views grow into free space and keep their content as a child.
    let tree = App::new(ScrollView::new(VStack::new(list![Text::new("Row")]))).render_tree();
    assert!(matches!(
        tree.value,
        RenderNode::View {
            kind: ViewKind::ScrollView { .. },
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
        RenderNode::View {
            kind: ViewKind::ScrollView {
                offset: Some((0.0, 120.0)),
                ..
            },
            ..
        }
    ));
}

/// Draws a bar `width` points wide and counts hover entries in its own state.
#[derive(Clone)]
struct Meter {
    width: f64,
    entered: u32,
}

impl Canvas2DHandler for Meter {
    fn draw(&mut self, g: &mut Graphics2D<'_>, _frame: &Frame) {
        g.fill_rect(
            Rect::new(0.0, 0.0, self.width, f64::from(self.entered)),
            Color::BLACK,
        );
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        if let CanvasEvent::Pointer(PointerEvent {
            phase: PointerPhase::Entered,
            ..
        }) = event
        {
            self.entered += 1;
            cx.invalidate();
        }
        true
    }

    fn reconcile(&mut self, next: Self) -> Redraw {
        self.width = next.width;
        Redraw::Yes
    }
}

fn canvas_node(node: &RenderNode) -> zintl_ui_native::CanvasNode {
    match node {
        RenderNode::View {
            kind: ViewKind::Canvas2D { canvas, .. },
            ..
        } => canvas.clone(),
        _ => panic!("expected a canvas"),
    }
}

fn canvas_frame() -> Frame {
    Frame {
        bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
        safe_area: Rect::new(0.0, 0.0, 100.0, 100.0),
        scale: 2.0,
        time: 0.0,
        delta: 0.0,
    }
}

/// Renders a meter whose width comes from a Store it publishes to the test.
struct MeterView {
    width: Store<f64>,
    published: Rc<Cell<Option<Store<f64>>>>,
}

impl MeterView {
    fn mount() -> (App, Store<f64>) {
        let published = Rc::new(Cell::new(None));
        let app = App::new(Self {
            width: Store::default(),
            published: published.clone(),
        });
        (app, published.get().expect("mounting runs init"))
    }
}

impl View for MeterView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.width = cx.store(10.0);
        self.published.set(Some(self.width));
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        cx.watch(self.width, |width| {
            Canvas2D::new(Meter {
                width: *width,
                entered: 0,
            })
            .redraw(RedrawPolicy::Continuous {
                preferred_fps: Some(60.0),
            })
            .focusable(true)
            .fill_width()
        })
    }
}

#[test]
fn canvas_declares_redraw_focus_and_layout() {
    // Verifies the builder records its redraw policy, focusability, and layout on the node.
    let (app, _) = MeterView::mount();
    assert!(matches!(
        app.render(),
        RenderNode::View {
            kind: ViewKind::Canvas2D {
                redraw: RedrawPolicy::Continuous {
                    preferred_fps: Some(60.0)
                },
                focusable: true,
                ..
            },
            layout: LayoutStyle {
                width: LayoutDimension::Percent(1.0),
                ..
            },
            ..
        }
    ));
}

#[test]
fn canvas_keeps_handler_state_across_store_updates() {
    // Verifies a Store-driven re-render updates the width while handler-owned hover state survives.
    let (mut app, width) = MeterView::mount();
    let mounted = canvas_node(&app.render());
    mounted.event(&CanvasEvent::Pointer(PointerEvent {
        phase: PointerPhase::Entered,
        button: None,
        location: Point::new(5.0, 5.0),
        modifiers: Modifiers::default(),
        click_count: 0,
    }));

    app.set(width, 40.0);

    let current = canvas_node(&app.render());
    assert_eq!(current, mounted);
    assert_eq!(
        current.record(&canvas_frame()),
        [graphics::DrawCommand::Fill {
            path: Path::rect(Rect::new(0.0, 0.0, 40.0, 1.0)),
            paint: Paint::Solid(Color::BLACK),
            rule: FillRule::NonZero,
        }]
    );
}

#[test]
fn split_view_divider_drag_resizes_the_first_pane_within_its_range() {
    // Verifies dragging the divider moves it by the pointer's distance from the grab point and clamps it.
    let (mut app, width) = Harness::mount(
        |cx| cx.store(200.0_f32),
        |width| {
            SplitView::horizontal(Text::new("List"), Text::new("Detail"))
                .range(100.0..=300.0)
                .bind(width)
        },
    );
    let pointer = |phase, x| {
        Event::Canvas(CanvasEvent::Pointer(PointerEvent {
            phase,
            button: Some(PointerButton::Primary),
            location: graphics::Point::new(x, 10.0),
            modifiers: Modifiers::default(),
            click_count: 1,
        }))
    };
    let first_width = |app: &App| match &app.render_tree().children[0].value {
        RenderNode::View { layout, .. } => layout.width,
        other => panic!("expected the first pane, got {other:?}"),
    };

    // The divider is the second child of the root stack.
    app.dispatch_at(&[0, 1], pointer(PointerPhase::Down, 3.0));
    app.dispatch_at(&[0, 1], pointer(PointerPhase::Moved, 23.0));
    assert_eq!(app.get(width), 220.0);
    assert_eq!(first_width(&app), LayoutDimension::Points(220.0));

    app.dispatch_at(&[0, 1], pointer(PointerPhase::Moved, 503.0));
    assert_eq!(app.get(width), 300.0);

    // Moves after the button is released do not resize.
    app.dispatch_at(&[0, 1], pointer(PointerPhase::Up, 503.0));
    app.dispatch_at(&[0, 1], pointer(PointerPhase::Moved, 0.0));
    assert_eq!(app.get(width), 300.0);
}

#[test]
fn split_view_starts_at_its_position_inside_the_range() {
    // Verifies an unbound split view starts at its declared size, clamped to the range.
    let app = App::new(
        SplitView::vertical(Text::new("Top"), Text::new("Bottom"))
            .position(40.0)
            .range(80.0..=200.0),
    );

    let RenderNode::View { layout, .. } = &app.render_tree().children[0].value else {
        panic!("expected the first pane");
    };
    assert_eq!(layout.height, LayoutDimension::Points(80.0));
}

#[test]
fn list_describes_sections_and_writes_user_selection() {
    // Verifies a list renders its sections and selection, one child per row, and reports user picks.
    let picked = Rc::new(RefCell::new(Vec::new()));
    let recorded = picked.clone();
    let (mut app, selection) = Harness::mount(
        |cx| cx.store(Some("b".to_owned())),
        move |selection| {
            let recorded = recorded.clone();
            List::new([
                ListSection::new([ListRow::new("a", Text::new("A"))]).title("Pinned"),
                ListSection::new([
                    ListRow::new("b", Text::new("B")),
                    ListRow::new("c", Text::new("C")),
                ]),
            ])
            .bind(selection)
            .on_select(move |_, id| recorded.borrow_mut().push(id.map(str::to_owned)))
        },
    );

    let tree = app.render_tree();
    let RenderNode::View {
        kind: ViewKind::List {
            sections, selected, ..
        },
        ..
    } = &tree.value
    else {
        panic!("expected a list, got {:?}", tree.value);
    };
    assert_eq!(sections[0].title.as_deref(), Some("Pinned"));
    assert_eq!(sections[1].ids, ["b", "c"]);
    assert_eq!(selected.as_deref(), Some("b"));
    assert_eq!(tree.children.len(), 3);

    assert!(app.dispatch_at(
        &[0],
        Event::ListSelectionChanged {
            id: Some("c".into())
        }
    ));
    assert_eq!(app.get(selection).as_deref(), Some("c"));
    assert!(app.dispatch_at(&[0], Event::ListSelectionChanged { id: None }));
    assert_eq!(app.get(selection), None);
    assert_eq!(*picked.borrow(), [Some("c".to_owned()), None]);
}

#[test]
fn text_field_updates_its_store_before_on_change() {
    // Verifies a field writes the bound Store, then passes the new text to `on_change`.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let recorded = seen.clone();
    let (mut app, _) = Harness::mount(
        |cx| cx.store(String::new()),
        move |title| {
            let recorded = recorded.clone();
            TextField::new().bind(title).on_change(move |cx, text| {
                recorded
                    .borrow_mut()
                    .push((cx.get(title).clone(), text.to_owned()))
            })
        },
    );

    app.dispatch_at(&[0], Event::TextChanged { value: "T".into() });

    assert_eq!(*seen.borrow(), [("T".to_owned(), "T".to_owned())]);
}

#[test]
fn text_editor_updates_its_store_before_on_change() {
    // Verifies the editor renders styled text and stores native edits before `on_change` runs.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let recorded = seen.clone();
    let (mut app, body) = Harness::mount(
        |cx| {
            let mut text = AttributedText::from("Title");
            text.set_paragraph(0, ParagraphStyle::Title);
            cx.store(text)
        },
        move |body| {
            let recorded = recorded.clone();
            TextEditor::new()
                .bind(body)
                .on_change(move |cx| recorded.borrow_mut().push(cx.get(body).text().to_owned()))
        },
    );

    assert!(matches!(
        &app.render_tree().value,
        RenderNode::View { kind: ViewKind::TextEditor { text, .. }, .. }
            if text.paragraphs() == [ParagraphStyle::Title]
    ));
    let mut edited = AttributedText::from("Title\n");
    edited.set_paragraph(0, ParagraphStyle::Title);
    edited.push(
        "bold",
        SpanStyle {
            bold: true,
            ..SpanStyle::default()
        },
    );
    assert!(app.dispatch_at(
        &[0],
        Event::RichTextChanged {
            text: edited.clone()
        }
    ));

    assert_eq!(app.get(body), edited);
    assert_eq!(*seen.borrow(), ["Title\nbold"]);
}

#[test]
fn toolbar_describes_items_and_routes_clicks_and_search() {
    // Verifies toolbar items render with state, buttons run their actions, and search writes its Store.
    let clicks = Calls::default();
    let click = clicks.clone();
    let (mut app, query) = Harness::mount(
        |cx| cx.store(String::new()),
        move |query| {
            Window::new(Rect::new(0.0, 0.0, 400.0, 300.0), "Toolbar").toolbar(Toolbar::new([
                ToolbarItem::button("new", "New", "plus").on_click(click.action()),
                ToolbarItem::flexible_space(),
                ToolbarItem::search(query).placeholder("Search"),
            ]))
        },
    );

    let RenderNode::Window {
        toolbar: Some(toolbar),
        ..
    } = &app.render_tree().value
    else {
        panic!("expected a window with a toolbar");
    };
    assert_eq!(toolbar.items.len(), 3);
    assert!(matches!(
        &toolbar.items[2],
        zintl_ui_backend_core::ToolbarItem::Search { placeholder: Some(text), .. } if text == "Search"
    ));

    assert!(app.dispatch_at(&[0], Event::ToolbarItemClicked { id: "new".into() }));
    assert!(app.dispatch_at(
        &[0],
        Event::ToolbarSearchChanged {
            value: "roux".into()
        }
    ));
    assert_eq!(clicks.count(), 1);
    assert_eq!(app.get(query), "roux");
}

#[test]
fn sidebar_items_show_badges_and_route_their_menus() {
    // Verifies item badges render and item menu selections run actions through the window.
    let renames = Calls::default();
    let rename = renames.clone();
    let (mut app, ()) = Harness::mount(
        |_| (),
        move |()| {
            Window::new(Rect::new(0.0, 0.0, 400.0, 300.0), "Folders").sidebar(Sidebar::new([
                SidebarSection::new([SidebarItem::new("ideas", "Ideas")
                    .badge("3")
                    .context_menu([MenuItem::new("Rename").on_select(rename.action())])]),
            ]))
        },
    );

    let RenderNode::Window {
        sidebar: Some(sidebar),
        ..
    } = &app.render_tree().value
    else {
        panic!("expected a window with a sidebar");
    };
    let item = &sidebar.sections[0].items[0];
    assert_eq!(item.badge.as_deref(), Some("3"));
    assert_eq!(item.menu.len(), 1);
    let id = item.menu[0].id.clone().expect("custom items have IDs");

    assert!(app.dispatch_at(&[0], Event::MenuItemSelected { id }));
    assert_eq!(renames.count(), 1);
}
