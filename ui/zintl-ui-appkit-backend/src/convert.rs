//! Conversions between the shared descriptions and `zpd-appkit` values.

use zintl_ui_backend_core as core;
use zpd_appkit::{geometry, graphics, ui};

use zintl_ui_backend_core::graphics as shared;

/// Converts a shared description into the value `zpd-appkit` expects.
pub(crate) trait Native {
    type Output;

    fn native(&self) -> Self::Output;
}

/// Converts a `zpd-appkit` value back into its shared description.
pub(crate) trait Shared {
    type Output;

    fn shared(self) -> Self::Output;
}

/// Maps enums whose variants have the same names on both sides.
macro_rules! same_variants {
    ($from:ty => $to:ty { $($variant:ident),* $(,)? }) => {
        impl Native for $from {
            type Output = $to;

            fn native(&self) -> $to {
                match self {
                    $(Self::$variant => <$to>::$variant,)*
                }
            }
        }
    };
}

same_variants!(core::WindowStyle => ui::WindowStyle { Normal, Panel });
same_variants!(core::TitleBar => ui::TitleBar { Standard, Overlay, Hidden });
same_variants!(core::PopoverEdge => ui::PopoverEdge { Leading, Trailing, Top, Bottom });
same_variants!(core::AlertStyle => ui::AlertStyle { Warning, Informational, Critical });
same_variants!(core::AlertButtonRole => ui::AlertButtonRole { Normal, Cancel, Destructive });
same_variants!(core::FileDialogMode => ui::FileDialogMode { Open, Save });
same_variants!(core::CommandModifier => ui::CommandModifier { Cmd, Ctrl, Alt, Shift });
same_variants!(core::MenuRole => ui::MenuRole { Services, Windows, Help });
same_variants!(shared::FillRule => graphics::FillRule { NonZero, EvenOdd });
same_variants!(shared::LineCap => graphics::LineCap { Butt, Round, Square });
same_variants!(shared::LineJoin => graphics::LineJoin { Miter, Round, Bevel });
same_variants!(core::ButtonType => ui::ButtonType {
    MomentaryPushIn,
    PushOnPushOff,
    Checkbox,
    Radio,
});
same_variants!(core::BezelStyle => ui::BezelStyle {
    Push,
    FlexiblePush,
    Disclosure,
    Circular,
    Help,
    SmallSquare,
    Toolbar,
    AccessoryBarAction,
    AccessoryBar,
    PushDisclosure,
    Badge,
});
same_variants!(core::CommandRole => ui::CommandRole {
    About,
    Quit,
    Hide,
    HideOthers,
    ShowAll,
    Close,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    ToggleSidebar,
    ToggleFullScreen,
    Minimize,
    Zoom,
    BringAllToFront,
    Help,
    Separator,
});

impl Native for core::Rect {
    type Output = geometry::Rect;

    fn native(&self) -> geometry::Rect {
        geometry::Rect::new(self.x, self.y, self.width, self.height)
    }
}

impl Native for shared::Point {
    type Output = geometry::Point;

    fn native(&self) -> geometry::Point {
        geometry::Point::new(self.x, self.y)
    }
}

impl Native for shared::Transform {
    type Output = graphics::AffineTransform;

    fn native(&self) -> graphics::AffineTransform {
        graphics::AffineTransform {
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d,
            tx: self.tx,
            ty: self.ty,
        }
    }
}

impl Native for shared::GradientStop {
    type Output = graphics::GradientStop;

    fn native(&self) -> graphics::GradientStop {
        graphics::GradientStop {
            offset: self.offset,
            color: self.color.native(),
        }
    }
}

impl Native for core::RedrawPolicy {
    type Output = ui::CanvasRedraw;

    fn native(&self) -> ui::CanvasRedraw {
        match *self {
            Self::OnDemand => ui::CanvasRedraw::OnDemand,
            Self::Continuous { preferred_fps } => ui::CanvasRedraw::Continuous { preferred_fps },
        }
    }
}

impl Shared for ui::Modifiers {
    type Output = core::Modifiers;

    fn shared(self) -> core::Modifiers {
        core::Modifiers {
            shift: self.shift,
            control: self.control,
            alt: self.option,
            meta: self.command,
        }
    }
}

impl Shared for ui::MouseButton {
    type Output = core::PointerButton;

    fn shared(self) -> core::PointerButton {
        match self {
            Self::Left => core::PointerButton::Primary,
            Self::Right => core::PointerButton::Secondary,
            Self::Middle => core::PointerButton::Middle,
            Self::Other(number) => core::PointerButton::Other(number),
        }
    }
}

impl Shared for ui::NamedKey {
    type Output = core::NamedKey;

    fn shared(self) -> core::NamedKey {
        use core::NamedKey as Key;

        match self {
            Self::Return => Key::Enter,
            Self::Tab => Key::Tab,
            Self::Space => Key::Space,
            Self::Delete => Key::Backspace,
            Self::ForwardDelete => Key::Delete,
            Self::Escape => Key::Escape,
            Self::LeftArrow => Key::ArrowLeft,
            Self::RightArrow => Key::ArrowRight,
            Self::UpArrow => Key::ArrowUp,
            Self::DownArrow => Key::ArrowDown,
            Self::Home => Key::Home,
            Self::End => Key::End,
            Self::PageUp => Key::PageUp,
            Self::PageDown => Key::PageDown,
            Self::Function(number) => Key::F(number),
        }
    }
}

impl Shared for ui::CanvasInput {
    type Output = core::CanvasEvent;

    fn shared(self) -> core::CanvasEvent {
        let point = |point: geometry::Point| shared::Point::new(point.x, point.y);
        match self {
            Self::Mouse(mouse) => core::CanvasEvent::Pointer(core::PointerEvent {
                phase: match mouse.phase {
                    ui::MousePhase::Down => core::PointerPhase::Down,
                    ui::MousePhase::Up => core::PointerPhase::Up,
                    ui::MousePhase::Moved => core::PointerPhase::Moved,
                    ui::MousePhase::Entered => core::PointerPhase::Entered,
                    ui::MousePhase::Exited => core::PointerPhase::Exited,
                },
                button: mouse.button.map(Shared::shared),
                location: point(mouse.location),
                modifiers: mouse.modifiers.shared(),
                click_count: mouse.click_count,
            }),
            Self::Scroll(scroll) => core::CanvasEvent::Scroll(core::ScrollEvent {
                location: point(scroll.location),
                delta_x: scroll.delta_x,
                delta_y: scroll.delta_y,
                precise: scroll.precise,
                modifiers: scroll.modifiers.shared(),
            }),
            Self::Key(key) => core::CanvasEvent::Key(core::KeyEvent {
                phase: if key.down {
                    core::KeyPhase::Down
                } else {
                    core::KeyPhase::Up
                },
                key: match key.named {
                    Some(named) => core::Key::Named(named.shared()),
                    None if key.unmodified.is_empty() => core::Key::Unidentified,
                    None => core::Key::Character(key.unmodified),
                },
                text: key.text,
                modifiers: key.modifiers.shared(),
                repeat: key.repeat,
                code: u32::from(key.key_code),
            }),
            Self::ModifiersChanged(modifiers) => {
                core::CanvasEvent::ModifiersChanged(modifiers.shared())
            }
            Self::Focus(focused) => core::CanvasEvent::Focus(focused),
        }
    }
}

impl Shared for geometry::Rect {
    type Output = core::Rect;

    fn shared(self) -> core::Rect {
        core::Rect::new(self.x, self.y, self.width, self.height)
    }
}

impl Native for core::Color {
    type Output = ui::Color;

    fn native(&self) -> ui::Color {
        ui::Color::new(self.red, self.green, self.blue, self.alpha)
    }
}

impl Shared for ui::Color {
    type Output = core::Color;

    fn shared(self) -> core::Color {
        core::Color::rgba(self.red, self.green, self.blue, self.alpha)
    }
}

impl Native for core::NumberFormat {
    type Output = ui::NumberFormat;

    fn native(&self) -> ui::NumberFormat {
        ui::NumberFormat {
            minimum: self.minimum,
            maximum: self.maximum,
            step: self.step,
            fraction_digits: self.fraction_digits,
        }
    }
}

impl Native for core::ScrollAxes {
    type Output = ui::ScrollAxes;

    fn native(&self) -> ui::ScrollAxes {
        ui::ScrollAxes {
            horizontal: self.horizontal,
            vertical: self.vertical,
        }
    }
}

impl Native for core::Sidebar {
    type Output = ui::Sidebar;

    fn native(&self) -> ui::Sidebar {
        ui::Sidebar {
            sections: self
                .sections
                .iter()
                .map(|section| ui::SidebarSection {
                    title: section.title.clone(),
                    items: section
                        .items
                        .iter()
                        .map(|item| ui::SidebarItem {
                            id: item.id.clone(),
                            title: item.title.clone(),
                            system_image: item.system_image.clone(),
                        })
                        .collect(),
                })
                .collect(),
            selected_id: self.selected_id.clone(),
            search: self.search.as_ref().map(|search| ui::SidebarSearch {
                value: search.value.clone(),
                placeholder: search.placeholder.clone(),
            }),
        }
    }
}

impl Native for core::CommandItem {
    type Output = ui::CommandItem;

    fn native(&self) -> ui::CommandItem {
        ui::CommandItem {
            id: self.id.clone(),
            title: self.title.clone(),
            role: self.role.as_ref().map(Native::native),
            key: self.key.clone(),
            modifiers: self.modifiers.iter().map(Native::native).collect(),
            enabled: self.enabled,
            checked: self.checked,
            submenu: self.submenu.as_ref().map(Native::native),
        }
    }
}

impl Native for core::CommandMenu {
    type Output = ui::CommandMenu;

    fn native(&self) -> ui::CommandMenu {
        ui::CommandMenu {
            title: self.title.clone(),
            items: self.items.iter().map(Native::native).collect(),
            role: self.role.as_ref().map(Native::native),
        }
    }
}

impl Native for core::CommandSet {
    type Output = ui::CommandSet;

    fn native(&self) -> ui::CommandSet {
        ui::CommandSet {
            app_menu: self.app_menu.as_ref().map(|menu| ui::WindowAppMenu {
                items: menu.items.iter().map(Native::native).collect(),
            }),
            menus: self.menus.iter().map(Native::native).collect(),
        }
    }
}

impl Native for core::AlertConfig {
    type Output = ui::AlertConfig;

    fn native(&self) -> ui::AlertConfig {
        ui::AlertConfig {
            style: self.style.native(),
            title: self.title.clone(),
            message: self.message.clone(),
            buttons: self
                .buttons
                .iter()
                .map(|button| ui::AlertButton {
                    title: button.title.clone(),
                    role: button.role.native(),
                })
                .collect(),
        }
    }
}

impl Native for core::FileDialogConfig {
    type Output = ui::FileDialogConfig;

    fn native(&self) -> ui::FileDialogConfig {
        ui::FileDialogConfig {
            mode: self.mode.native(),
            title: self.title.clone(),
            message: self.message.clone(),
            prompt: self.prompt.clone(),
            allowed_extensions: self.allowed_extensions.clone(),
            can_choose_files: self.can_choose_files,
            can_choose_directories: self.can_choose_directories,
            allows_multiple: self.allows_multiple,
            can_create_directories: self.can_create_directories,
            file_name: self.file_name.clone(),
            directory: self.directory.clone(),
        }
    }
}

impl Shared for zpd_appkit::actor::WindowEventKind {
    type Output = core::Event;

    fn shared(self) -> core::Event {
        use core::Event as Shared;
        use zpd_appkit::actor::WindowEventKind as Native;

        match self {
            Native::Created => Shared::WindowCreated,
            Native::DidResize => Shared::WindowDidResize,
            Native::WillClose => Shared::WindowWillClose,
            Native::DidClose => Shared::WindowDidClose,
            Native::ButtonClicked => Shared::Activated,
            Native::TextChanged { value } => Shared::TextChanged { value },
            Native::SidebarSelectionChanged { id } => Shared::SidebarSelectionChanged { id },
            Native::SidebarSearchChanged { value } => Shared::SidebarSearchChanged { value },
            Native::StateChanged { on } => Shared::StateChanged { on },
            Native::ValueChanged { value } => Shared::ValueChanged { value },
            Native::SelectionChanged { index } => Shared::SelectionChanged { index },
            Native::ColorChanged { color } => Shared::ColorChanged {
                color: color.shared(),
            },
            Native::Scrolled { x, y } => Shared::Scrolled { x, y },
            Native::MenuItemSelected { id } => Shared::MenuItemSelected { id },
            Native::PopoverClosed => Shared::PopoverClosed,
            Native::AlertResponse { button } => Shared::AlertResponse { button },
            Native::FileDialogCompleted { paths } => Shared::FileDialogCompleted {
                paths: paths.map(|paths| paths.into_iter().map(std::path::PathBuf::from).collect()),
            },
            Native::ColorPanelClosed => Shared::ColorPanelClosed,
            Native::CanvasInput(input) => Shared::Canvas(input.shared()),
        }
    }
}
