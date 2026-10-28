use std::error::Error;

use zpd_appkit::actor::WindowEventKind;
use zpd_appkit::runloop::Application;
use zpd_appkit::ui::{
    AsView, Button, CommandItem, CommandMenu, CommandModifier, CommandRole, CommandSet,
    LayoutConstraint, Sidebar, SidebarItem, SidebarSection, TextField, WindowAppMenu,
};

fn main() -> Result<(), Box<dyn Error>> {
    let application = Application::new()?;
    application.set_commands(&commands(), |command_id| {
        if command_id == "example.hello" {
            println!("Hello from zpd-appkit!");
        }
    })?;

    let window = application.create_window()?;
    window.set_size(640.0, 400.0)?;

    let content = window.content_view()?;
    let label = TextField::label_with_string(&application, "Name")?;
    let input = TextField::with_string(&application, "")?;
    let button = Button::with_title(&application, "Save")?;

    input.set_placeholder_string(Some("Your name"))?;

    content.add_subview(&label)?;
    content.add_subview(&input)?;
    content.add_subview(&button)?;

    let (label_view, input_view, button_view) =
        (label.as_view(), input.as_view(), button.as_view());
    label_view.set_translates_autoresizing_mask_into_constraints(false)?;
    input_view.set_translates_autoresizing_mask_into_constraints(false)?;
    button_view.set_translates_autoresizing_mask_into_constraints(false)?;

    let constraints = [
        label_view
            .leading_anchor()
            .constraint_equal_to(content.leading_anchor(), 20.0)?,
        label_view
            .center_y_anchor()
            .constraint_equal_to(input_view.center_y_anchor(), 0.0)?,
        input_view
            .leading_anchor()
            .constraint_equal_to(label_view.trailing_anchor(), 8.0)?,
        input_view
            .trailing_anchor()
            .constraint_equal_to(content.trailing_anchor(), -20.0)?,
        input_view
            .top_anchor()
            .constraint_equal_to(content.top_anchor(), 20.0)?,
        button_view
            .top_anchor()
            .constraint_equal_to(input_view.bottom_anchor(), 12.0)?,
        button_view
            .trailing_anchor()
            .constraint_equal_to(input_view.trailing_anchor(), 0.0)?,
    ];
    LayoutConstraint::activate(&constraints)?;
    drop(constraints);

    let button_id = button.as_view().actor_ref().actor_id();
    let _events = application.on(move |event| {
        if event.target == button_id
            && matches!(event.kind, WindowEventKind::ButtonClicked)
            && let Ok(value) = input.string_value()
        {
            println!("{value}");
        }
    })?;

    window.set_sidebar(
        &Sidebar {
            sections: vec![
                SidebarSection {
                    title: Some("Library".into()),
                    items: vec![
                        SidebarItem {
                            id: "home".into(),
                            title: "Home".into(),
                            system_image: Some("house".into()),
                        },
                        SidebarItem {
                            id: "recent".into(),
                            title: "Recent".into(),
                            system_image: Some("clock".into()),
                        },
                    ],
                },
                SidebarSection {
                    title: Some("Workspace".into()),
                    items: vec![SidebarItem {
                        id: "projects".into(),
                        title: "Projects".into(),
                        system_image: Some("square.grid.2x2".into()),
                    }],
                },
            ],
            selected_id: Some("home".into()),
            search: None,
        },
        move |item_id| {
            let _ = label.set_string_value(&format!("Selected: {item_id}"));
        },
    )?;

    window.show()?;

    application.run()?;
    Ok(())
}

fn commands() -> CommandSet {
    CommandSet {
        app_menu: Some(WindowAppMenu {
            items: vec![
                CommandItem {
                    id: None,
                    title: "About zpd-appkit Example".into(),
                    role: Some(CommandRole::About),
                    key: None,
                    modifiers: Vec::new(),
                    enabled: true,
                    checked: false,
                    submenu: None,
                },
                CommandItem {
                    id: None,
                    title: "Quit zpd-appkit Example".into(),
                    role: Some(CommandRole::Quit),
                    key: Some("q".into()),
                    modifiers: vec![CommandModifier::Cmd],
                    enabled: true,
                    checked: false,
                    submenu: None,
                },
            ],
        }),
        menus: vec![CommandMenu {
            title: "Example".into(),
            role: None,
            items: vec![CommandItem {
                id: Some("example.hello".into()),
                title: "Print Hello".into(),
                role: None,
                key: Some("h".into()),
                modifiers: vec![CommandModifier::Cmd],
                enabled: true,
                checked: false,
                submenu: None,
            }],
        }],
    }
}
