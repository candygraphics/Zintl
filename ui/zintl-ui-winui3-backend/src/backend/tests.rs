use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::RenderBackend;
use zintl_ui_backend_core::{
    AlertButton, AlertButtonRole, AlertConfig, Button, ButtonType, CommandItem, CommandMenu,
    CommandModifier, CommandRole, CommandSet, FileDialogConfig, MenuRole, WindowAppMenu,
};
use zintl_ui_layout::LayoutStyle;
use zpd_winui3::{MenuFlyoutItemBase, MenuItemRole, VirtualKeyModifiers};

use super::*;

/// Builds the render nodes the tests insert.
struct Nodes;

impl Nodes {
    fn container() -> RenderNode {
        RenderNode::View {
            kind: ViewKind::Container,
            layout: LayoutStyle::leaf(Size::new(0.0, 0.0)),
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
        }
    }

    fn window(style: WindowStyle) -> RenderNode {
        RenderNode::Window {
            sidebar: None,
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            title: String::new(),
            title_bar: zintl_ui_backend_core::TitleBar::Standard,
            id: None,
            style,
            toolbar: None,
            autosave: None,
            document: Default::default(),
        }
    }
}

type Backend = WinUI3Backend;

fn command(title: &str, id: &str) -> CommandItem {
    CommandItem {
        id: Some(id.into()),
        ..CommandItem::new(title)
    }
}

fn role(title: &str, role: CommandRole) -> CommandItem {
    CommandItem {
        role: Some(role),
        ..CommandItem::new(title)
    }
}

#[test]
fn only_normal_windows_keep_the_app_running() {
    // Verifies panels alone do not keep the app alive once a window has been shown.
    let mut backend = Backend::new();
    let panel = backend.create(&Nodes::window(WindowStyle::Panel), None);
    backend.insert_child(backend.root(), 0, panel);
    assert!(!backend.should_terminate());

    backend.opened_window = true;
    assert!(backend.should_terminate());

    let window = backend.create(&Nodes::window(WindowStyle::Normal), None);
    backend.insert_child(backend.root(), 1, window);
    assert!(!backend.should_terminate());
}

#[test]
fn removing_a_parent_removes_its_descendants() {
    // Verifies renderer removal cannot leave detached descendants in backend storage.
    let mut backend = Backend::new();
    let parent = backend.create(&Nodes::container(), None);
    let child = backend.create(&Nodes::container(), None);
    backend.insert_child(backend.root(), 0, parent);
    backend.insert_child(parent, 0, child);

    backend.remove(parent);

    assert!(!backend.tree.contains(parent));
    assert!(!backend.tree.contains(child));
    assert!(backend.children(backend.root()).is_empty());
}

#[test]
fn event_routes_update_window_and_attachment_cells() {
    // Verifies native callbacks read the route the Composer assigned most recently.
    let mut backend = Backend::new();
    let node = backend.create(&Nodes::container(), Some(EventRouteId::from_raw(3)));
    let cell = backend.state(node).route.clone();
    let route = EventRouteId::from_raw(17);

    backend.set_event_route(node, Some(route));

    assert_eq!(backend.event_route(node), Some(route));
    assert_eq!(cell.get(), Some(route));
}

#[test]
fn app_menu_items_join_the_file_menu() {
    // Verifies Windows apps get Settings and Exit in File, without macOS-only items.
    let commands = CommandSet {
        app_menu: Some(WindowAppMenu {
            items: vec![
                command("Settings…", "settings"),
                CommandItem::separator(),
                role("Hide", CommandRole::Hide),
                CommandItem {
                    submenu: Some(CommandMenu {
                        title: "Services".into(),
                        items: Vec::new(),
                        role: Some(MenuRole::Services),
                    }),
                    ..CommandItem::new("Services")
                },
                CommandItem::separator(),
                role("Quit Gallery", CommandRole::Quit),
            ],
        }),
        menus: vec![
            CommandMenu {
                title: "File".into(),
                items: vec![command("Show Inspector", "inspector")],
                role: None,
            },
            CommandMenu {
                title: "Window".into(),
                items: vec![role("Minimize", CommandRole::Minimize)],
                role: Some(MenuRole::Windows),
            },
        ],
    };

    let bar = Menus::menu_bar(&commands);

    assert_eq!(bar.items.len(), 1, "the empty Window menu is dropped");
    let titles = bar.items[0]
        .items
        .iter()
        .map(|item| match item {
            MenuFlyoutItemBase::Item(item) => item.text.as_str(),
            MenuFlyoutItemBase::SubItem(item) => item.text.as_str(),
            MenuFlyoutItemBase::Separator => "-",
        })
        .collect::<Vec<_>>();
    assert_eq!(titles, ["Show Inspector", "-", "Settings…", "-", "Exit"]);
    let MenuFlyoutItemBase::Item(exit) = bar.items[0].items.last().unwrap() else {
        panic!("Exit must be an item");
    };
    assert_eq!(exit.id, Menus::QUIT);
}

#[test]
fn editing_roles_and_shortcuts_map_to_native_values() {
    // Verifies Copy uses the native role and Command shortcuts become Control.
    let mut copy = role("Copy", CommandRole::Copy);
    copy.key = Some("c".into());
    copy.modifiers = vec![CommandModifier::Cmd, CommandModifier::Shift];
    let mut arrow = command("Next", "next");
    arrow.key = Some("\u{F703}".into());
    let mut settings = command("Settings", "settings");
    settings.key = Some(",".into());

    let items = Menus::items(&[copy, arrow, settings]);

    let MenuFlyoutItemBase::Item(copy) = &items[0] else {
        panic!("Copy must be an item");
    };
    assert_eq!(copy.role, MenuItemRole::Copy);
    assert!(copy.is_enabled);
    let accelerator = copy.keyboard_accelerator.as_ref().unwrap();
    assert_eq!(accelerator.key, "C");
    assert_eq!(
        accelerator.modifiers,
        VirtualKeyModifiers::CONTROL | VirtualKeyModifiers::SHIFT
    );
    // WinUI crashes when a menu shows an OEM-key accelerator, so these are dropped.
    for item in &items[1..] {
        let MenuFlyoutItemBase::Item(item) = item else {
            panic!("shortcut items must be kept");
        };
        assert!(
            item.keyboard_accelerator.is_none(),
            "{} keeps no shortcut",
            item.text
        );
    }
}

#[test]
fn items_without_actions_are_disabled() {
    // Verifies a menu item with neither an id nor a native role cannot be chosen.
    let items = Menus::items(&[CommandItem::new("Disabled Item")]);
    let MenuFlyoutItemBase::Item(item) = &items[0] else {
        panic!("the item must be kept");
    };
    assert!(!item.is_enabled);
}

#[test]
fn separators_are_trimmed_and_collapsed() {
    // Verifies removed macOS items do not leave stray separators behind.
    let items = Menus::items(&[
        CommandItem::separator(),
        command("A", "a"),
        CommandItem::separator(),
        role("Zoom", CommandRole::Zoom),
        CommandItem::separator(),
        command("B", "b"),
        CommandItem::separator(),
    ]);
    assert_eq!(items.len(), 3);
    assert!(matches!(items[1], MenuFlyoutItemBase::Separator));
}

#[test]
fn alert_buttons_map_to_dialog_buttons() {
    // Verifies the first action is primary and the cancel button closes the dialog.
    let config = AlertConfig {
        title: "Delete?".into(),
        buttons: vec![
            AlertButton {
                title: "Delete".into(),
                role: AlertButtonRole::Destructive,
            },
            AlertButton {
                title: "Cancel".into(),
                role: AlertButtonRole::Cancel,
            },
        ],
        ..AlertConfig::default()
    };

    let (dialog, indices) = Backend::dialog(&config);

    assert_eq!(dialog.primary, "Delete");
    assert_eq!(dialog.close, "Cancel");
    assert!(dialog.secondary.is_empty());
    assert_eq!(dialog.default_button, Some(DialogResult::Primary));
    assert_eq!(indices, [Some(0), None, Some(1)]);
}

#[test]
fn alerts_without_buttons_offer_ok() {
    // Verifies an alert always has a way to be dismissed.
    let (dialog, indices) = Backend::dialog(&AlertConfig::default());
    assert_eq!(dialog.close, "OK");
    assert_eq!(indices, [None, None, Some(0)]);
}

#[test]
fn directory_only_open_panels_pick_folders() {
    // Verifies choosing directories alone uses the folder picker.
    let config = FileDialogConfig {
        can_choose_directories: true,
        ..FileDialogConfig::default()
    };
    assert_eq!(
        Backend::file_dialog(&config).mode,
        zpd_winui3::FileDialogMode::Folder
    );
}

#[test]
fn glyph_table_is_sorted_for_lookup() {
    // Verifies binary search finds every entry and unknown names show no icon.
    assert!(Glyph::TABLE.windows(2).all(|pair| pair[0].0 < pair[1].0));
    for (name, glyph) in Glyph::TABLE {
        assert_eq!(Glyph::of(name), Some(*glyph));
    }
    assert_eq!(Glyph::of("not.a.symbol"), None);
}

fn label(line_limit: Option<usize>) -> ViewKind {
    ViewKind::TextField {
        value: "Hello".into(),
        placeholder: None,
        editable: false,
        selectable: false,
        bordered: false,
        draws_background: false,
        multiline: false,
        style: zintl_ui_backend_core::TextStyle {
            line_limit,
            ..Default::default()
        },
    }
}

#[test]
fn view_kinds_choose_matching_controls() {
    // Verifies labels, fields, editors, indicators, and two-state buttons use distinct WinUI controls.
    let field = |editable| ViewKind::TextField {
        value: String::new(),
        placeholder: None,
        editable,
        selectable: true,
        bordered: true,
        draws_background: true,
        multiline: false,
        style: Default::default(),
    };
    assert_eq!(
        NativeNode::control_kind(&field(true)),
        Some(ControlKind::TextBox)
    );
    assert_eq!(
        NativeNode::control_kind(&field(false)),
        Some(ControlKind::TextBlock)
    );
    let button = |button_type| {
        ViewKind::Button(Button {
            title: String::new(),
            button_type,
            bezel: None,
            on: false,
            system_image: None,
            key_equivalent: None,
            destructive: false,
            shows_menu: false,
            keeps_state: false,
            title_color: None,
        })
    };
    assert_eq!(
        NativeNode::control_kind(&button(ButtonType::Checkbox)),
        Some(ControlKind::CheckBox)
    );
    assert_eq!(
        NativeNode::control_kind(&button(ButtonType::PushOnPushOff)),
        Some(ControlKind::ToggleButton)
    );
    let editor = ViewKind::TextEditor {
        text: Default::default(),
        focus: 0,
    };
    assert_eq!(
        NativeNode::control_kind(&editor),
        Some(ControlKind::RichEditBox)
    );
    assert_eq!(
        NativeNode::control_kind(&ViewKind::ProgressIndicator { value: None }),
        Some(ControlKind::ProgressRing)
    );
    assert_eq!(
        NativeNode::control_kind(&ViewKind::ProgressIndicator { value: Some(0.5) }),
        Some(ControlKind::ProgressBar)
    );
    assert_eq!(
        NativeNode::control_kind(&ViewKind::Separator),
        Some(ControlKind::Separator)
    );
}

#[test]
fn canvases_have_no_control_kind() {
    // Verifies canvases are created as SwapChainPanels rather than controls.
    #[derive(Clone)]
    struct Blank;
    impl zintl_ui_backend_core::Canvas2DHandler for Blank {
        fn draw(
            &mut self,
            _g: &mut zintl_ui_backend_core::graphics::Graphics2D<'_>,
            _frame: &zintl_ui_backend_core::Frame,
        ) {
        }
    }
    let canvas = ViewKind::Canvas2D {
        canvas: zintl_ui_backend_core::CanvasNode::new(Blank),
        redraw: Default::default(),
        focusable: false,
        reports_input: false,
    };
    assert_eq!(NativeNode::control_kind(&canvas), None);
}

#[test]
fn control_minimums_fit_winui_templates() {
    // Verifies AppKit-sized leaves grow to WinUI's control heights but keep larger sizes.
    let switch = ViewKind::Switch { on: false };
    assert_eq!(
        NativeNode::minimum_for(&switch, Size::new(38.0, 22.0), None),
        Size::new(44.0, 28.0)
    );
    assert_eq!(
        NativeNode::minimum_for(&ViewKind::Container, Size::new(10.0, 10.0), None),
        Size::new(10.0, 10.0)
    );
}

#[test]
fn labels_take_their_measured_width_unless_line_limited() {
    // Verifies a label grows to its measured text, while a line-limited one keeps its declared
    // minimum and prefers the measured width as its content size so it can truncate.
    let declared = Size::new(10.0, 16.0);
    assert_eq!(
        NativeNode::minimum_for(&label(None), declared, Some(37.0)),
        Size::new(37.0, 20.0)
    );
    assert_eq!(
        NativeNode::minimum_for(&label(Some(1)), declared, Some(37.0)),
        Size::new(10.0, 20.0)
    );
    assert_eq!(
        NativeNode::content_for(&label(Some(1)), Some(Size::new(80.0, 16.0)), Some(37.0)),
        Some(Size::new(37.0, 16.0))
    );
    assert_eq!(
        NativeNode::content_for(&label(None), Some(Size::new(80.0, 16.0)), Some(37.0)),
        Some(Size::new(80.0, 16.0))
    );
}

#[test]
fn size_changes_stay_local_and_sidebar_events_are_routed() {
    // Verifies layout notifications never reach the Composer as user events.
    assert_eq!(
        NativeNode::events(ControlEvent::SizeChanged {
            width: 1.0,
            height: 2.0,
            inset: 0.0,
        }),
        vec![]
    );
    assert_eq!(
        NativeNode::events(ControlEvent::NavigationSelected("home".into())),
        vec![Event::SidebarSelectionChanged { id: "home".into() }]
    );
}

#[test]
fn list_menu_and_commit_events_map_to_view_events() {
    // Verifies the new control events reach views as their platform-neutral counterparts.
    assert_eq!(
        NativeNode::events(ControlEvent::ListSelectionChanged(None)),
        vec![Event::ListSelectionChanged { id: None }]
    );
    assert_eq!(
        NativeNode::events(ControlEvent::MenuSelected("rename".into())),
        vec![Event::MenuItemSelected {
            id: "rename".into()
        }]
    );
    assert_eq!(
        NativeNode::events(ControlEvent::TextCommitted("42 kg".into())),
        vec![Event::TextCommitted {
            value: "42 kg".into()
        }]
    );
}

#[test]
fn toggle_clicks_report_activation_before_the_new_state() {
    // Verifies a toggle click runs `on_click` and then reports the new state, as on macOS.
    assert_eq!(
        NativeNode::events(ControlEvent::Toggled(true)),
        vec![Event::Activated, Event::StateChanged { on: true }]
    );
    // A button that keeps its declared state reports only the click.
    assert_eq!(
        NativeNode::events(ControlEvent::Clicked),
        vec![Event::Activated]
    );
}

#[test]
fn rich_text_round_trips_through_the_native_document() {
    // Verifies runs, traits, links, and every paragraph style survive the RichEditBox conversion.
    use zintl_ui_backend_core::graphics::{AttributedText, ParagraphStyle, SpanStyle};
    let bold = SpanStyle {
        bold: true,
        link: Some("https://example.com".into()),
        ..SpanStyle::default()
    };
    let text = AttributedText::from_segments([
        ("Title\n", SpanStyle::default(), ParagraphStyle::Title),
        ("bold", bold, ParagraphStyle::Checklist { done: true }),
        (
            " plain\n",
            SpanStyle::default(),
            ParagraphStyle::Checklist { done: true },
        ),
        ("", SpanStyle::default(), ParagraphStyle::Body),
    ]);
    let mut text = text;
    // An empty last paragraph keeps its own style.
    text.set_paragraph(2, ParagraphStyle::Numbered);
    let native = NativeNode::rich_text(&text);
    assert_eq!(
        native.paragraphs,
        [
            zpd_winui3::ParagraphStyle::Title,
            zpd_winui3::ParagraphStyle::ChecklistDone,
            zpd_winui3::ParagraphStyle::Numbered,
        ]
    );
    assert_eq!(NativeNode::attributed(&native), text);
}

#[test]
fn table_columns_share_the_width_left_by_fixed_ones() {
    // Verifies fixed columns keep their width and the others split the rest, but not below 40.
    let column = |width| zintl_ui_backend_core::TableColumn {
        title: String::new(),
        width,
    };
    assert_eq!(NativeNode::column_widths(&[], 300.0), [300.0]);
    assert_eq!(
        NativeNode::column_widths(&[column(Some(100.0)), column(None), column(None)], 300.0),
        [100.0, 100.0, 100.0]
    );
    assert_eq!(
        NativeNode::column_widths(&[column(Some(290.0)), column(None)], 300.0),
        [290.0, 40.0]
    );
}

#[test]
fn edited_documents_mark_the_title() {
    // Verifies unsaved changes show in the window title.
    let edited = zintl_ui_backend_core::DocumentState {
        edited: true,
        ..Default::default()
    };
    assert_eq!(Backend::displayed_title("Notes", &edited), "*Notes");
    assert_eq!(
        Backend::displayed_title("Notes", &Default::default()),
        "Notes"
    );
}

#[test]
fn list_sections_and_table_rows_rebuild_the_structure() {
    // Verifies only row-defining changes re-attach list rows; selection changes do not.
    let list = |ids: &[&str], selected: Option<&str>| ViewKind::List {
        sections: vec![zintl_ui_backend_core::ListSection {
            title: None,
            ids: ids.iter().map(|id| id.to_string()).collect(),
        }],
        selected: selected.map(str::to_owned),
        style: Default::default(),
    };
    assert!(Backend::rows_changed(
        &list(&["a"], None),
        &list(&["a", "b"], None)
    ));
    assert!(!Backend::rows_changed(
        &list(&["a"], None),
        &list(&["a"], Some("a"))
    ));
}

#[test]
fn colors_match_after_eight_bit_round_trips() {
    // Verifies picker rounding does not rewrite the color panel every update.
    let color = Color::rgba(0.2, 0.4, 0.6, 1.0);
    assert!(Backend::colors_match(
        color,
        Color::rgba(0.2 + 1.0 / 512.0, 0.4, 0.6, 1.0)
    ));
    assert!(!Backend::colors_match(
        color,
        Color::rgba(0.3, 0.4, 0.6, 1.0)
    ));
}

#[test]
fn formatting_roles_apply_to_the_focused_rich_editor() {
    // Verifies paragraph style and font trait items stay in WinUI menus as editing roles.
    let items = Menus::items(&[
        role("Bold", CommandRole::Bold),
        role("Title", CommandRole::TitleStyle),
        role("Checklist", CommandRole::Checklist),
    ]);
    let roles = items
        .iter()
        .map(|item| match item {
            MenuFlyoutItemBase::Item(item) => item.role,
            _ => MenuItemRole::None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        roles,
        [
            MenuItemRole::Bold,
            MenuItemRole::TitleStyle,
            MenuItemRole::Checklist
        ]
    );
}
