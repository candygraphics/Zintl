use zintl_ui::event::EventRouteId;
use zintl_ui::renderer::{RenderBackend, RenderNode};
use zintl_ui::view::{Context as ContextTrait, StoreContext};
use zintl_ui_appkit_backend::{
    AlertButton, AlertButtonRole, AlertConfig, Button, ButtonType, CommandItem, CommandMenu,
    CommandModifier, CommandRole, CommandSet, FileDialogConfig, MainTaskSender, MenuRole,
    WindowAppMenu,
};
use zintl_ui_layout::LayoutStyle;
use zpd_winui3::{MenuFlyoutItemBase, MenuItemRole, VirtualKeyModifiers};

use super::*;

#[derive(Clone, PartialEq)]
struct TestNode(NodeKind);

impl TestNode {
    fn container() -> Self {
        Self(NodeKind::View {
            kind: ViewKind::Container,
            layout: LayoutStyle::leaf(Size::new(0.0, 0.0)),
            id: None,
            enabled: true,
        })
    }

    fn window(style: WindowStyle) -> Self {
        Self(NodeKind::Window {
            sidebar: None,
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            title: String::new(),
            full_size_content_view: false,
            id: None,
            style,
        })
    }
}

#[derive(Clone)]
struct TestEvent;

struct TestContext<'a> {
    store_context: StoreContext<'a>,
}

impl<'a> ContextTrait<'a> for TestContext<'a> {
    fn store_context<'context>(&'context self) -> &'context StoreContext<'a> {
        &self.store_context
    }

    fn store_context_mut<'context>(&'context mut self) -> &'context mut StoreContext<'a> {
        &mut self.store_context
    }
}

impl zintl_ui::event::Event for TestEvent {
    type Kind = ();

    fn kind(&self) -> Self::Kind {}
}

impl RenderNode for TestNode {
    type Event = TestEvent;
    type Context<'a> = TestContext<'a>;

    fn same_kind(&self, _other: &Self) -> bool {
        true
    }
}

impl AppKitRenderNode for TestNode {
    fn create_context<'a>(
        store_context: StoreContext<'a>,
        _perform_main: Option<&'a MainTaskSender>,
    ) -> Self::Context<'a> {
        TestContext { store_context }
    }

    fn appkit_node(&self) -> NodeKind {
        self.0.clone()
    }

    fn appkit_event(_event: AppKitEvent) -> Self::Event {
        TestEvent
    }
}

type Backend = WinUI3Backend<TestNode>;

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
    let panel = backend.create(&TestNode::window(WindowStyle::Panel), None);
    backend.insert_child(backend.root(), 0, panel);
    assert!(!backend.should_terminate());

    backend.opened_window = true;
    assert!(backend.should_terminate());

    let window = backend.create(&TestNode::window(WindowStyle::Normal), None);
    backend.insert_child(backend.root(), 1, window);
    assert!(!backend.should_terminate());
}

#[test]
fn removing_a_parent_removes_its_descendants() {
    // Verifies renderer removal cannot leave detached descendants in backend storage.
    let mut backend = Backend::new();
    let parent = backend.create(&TestNode::container(), None);
    let child = backend.create(&TestNode::container(), None);
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
    let node = backend.create(&TestNode::container(), Some(EventRouteId::from_raw(3)));
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

#[test]
fn view_kinds_choose_matching_controls() {
    // Verifies labels, fields, and two-state buttons use distinct WinUI controls.
    let field = |editable| ViewKind::TextField {
        value: String::new(),
        placeholder: None,
        editable,
        selectable: true,
        bordered: true,
        draws_background: true,
        multiline: false,
    };
    assert_eq!(NativeNode::control_kind(&field(true)), ControlKind::TextBox);
    assert_eq!(
        NativeNode::control_kind(&field(false)),
        ControlKind::TextBlock
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
        })
    };
    assert_eq!(
        NativeNode::control_kind(&button(ButtonType::Checkbox)),
        ControlKind::CheckBox
    );
    assert_eq!(
        NativeNode::control_kind(&button(ButtonType::PushOnPushOff)),
        ControlKind::ToggleButton
    );
}

#[test]
fn control_minimums_fit_winui_templates() {
    // Verifies AppKit-sized leaves grow to WinUI's control heights but keep larger sizes.
    let switch = ViewKind::Switch { on: false };
    assert_eq!(
        NativeNode::minimum_size(&switch, Size::new(38.0, 22.0)),
        Size::new(44.0, 28.0)
    );
    assert_eq!(
        NativeNode::minimum_size(&ViewKind::Container, Size::new(10.0, 10.0)),
        Size::new(10.0, 10.0)
    );
}

#[test]
fn size_changes_stay_local_and_sidebar_events_are_routed() {
    // Verifies layout notifications never reach the Composer as user events.
    assert_eq!(
        NativeNode::event(ControlEvent::SizeChanged {
            width: 1.0,
            height: 2.0
        }),
        None
    );
    assert_eq!(
        NativeNode::event(ControlEvent::NavigationSelected("home".into())),
        Some(AppKitEvent::SidebarSelectionChanged { id: "home".into() })
    );
}

#[test]
fn colors_match_after_eight_bit_round_trips() {
    // Verifies picker rounding does not rewrite the color panel every update.
    let color = Color::new(0.2, 0.4, 0.6, 1.0);
    assert!(Backend::colors_match(
        color,
        Color::new(0.2 + 1.0 / 512.0, 0.4, 0.6, 1.0)
    ));
    assert!(!Backend::colors_match(
        color,
        Color::new(0.3, 0.4, 0.6, 1.0)
    ));
}
