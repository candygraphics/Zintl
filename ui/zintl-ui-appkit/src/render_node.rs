use std::path::PathBuf;

use zintl_ui::renderer::RenderNode as RenderNodeTrait;
use zintl_ui_layout::{LayoutStyle, Size};

#[cfg(target_os = "macos")]
use crate::MenuItemState;
use crate::{
    AlertStyle, BezelStyle, ButtonType, Color, Event, MenuState, PopoverEdge, ScrollAxes,
    SidebarState, WindowStyle,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Rendered alert data without button actions.
#[derive(Clone, Debug, PartialEq)]
pub struct AlertState {
    pub style: AlertStyle,
    pub title: String,
    pub message: Option<String>,
    /// Buttons from right to left with their roles.
    pub buttons: Vec<(String, crate::ButtonRole)>,
}

/// Rendered open or save panel options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileDialogState {
    pub title: Option<String>,
    pub message: Option<String>,
    pub prompt: Option<String>,
    pub allowed_extensions: Vec<String>,
    pub can_choose_files: bool,
    pub can_choose_directories: bool,
    pub allows_multiple: bool,
    pub can_create_directories: bool,
    pub file_name: Option<String>,
    pub directory: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenderNode {
    NSWindow {
        sidebar: Option<SidebarState>,
        bounds: Rect,
        title: String,
        full_size_content_view: bool,
        id: Option<String>,
        style: WindowStyle,
    },
    NSView {
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSButton {
        title: String,
        button_type: ButtonType,
        /// `None` draws a borderless button.
        bezel: Option<BezelStyle>,
        on: bool,
        system_image: Option<String>,
        key_equivalent: Option<String>,
        destructive: bool,
        enabled: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSTextField {
        value: String,
        placeholder: Option<String>,
        editable: bool,
        selectable: bool,
        bordered: bool,
        draws_background: bool,
        multiline: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSSwitch {
        on: bool,
        enabled: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSSlider {
        value: f64,
        minimum: f64,
        maximum: f64,
        tick_marks: usize,
        snaps_to_ticks: bool,
        enabled: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSPopUpButton {
        items: Vec<String>,
        selected: Option<usize>,
        enabled: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSSegmentedControl {
        segments: Vec<String>,
        selected: Option<usize>,
        enabled: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSComboBox {
        items: Vec<String>,
        value: String,
        placeholder: Option<String>,
        enabled: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    /// An `NSTextField` with a number formatter beside an `NSStepper`.
    NSNumberField {
        value: f64,
        minimum: f64,
        maximum: f64,
        step: f64,
        fraction_digits: usize,
        enabled: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSSearchField {
        value: String,
        placeholder: Option<String>,
        layout: LayoutStyle,
        id: Option<String>,
    },
    NSColorWell {
        color: Color,
        enabled: bool,
        layout: LayoutStyle,
        id: Option<String>,
    },
    /// Children are scrolled content.
    NSScrollView {
        axes: ScrollAxes,
        layout: LayoutStyle,
        id: Option<String>,
    },
    /// Each child `NSView` is the page for the title at the same index.
    NSTabView {
        titles: Vec<String>,
        selected: usize,
        layout: LayoutStyle,
        id: Option<String>,
    },
    /// A contextual menu for the parent view.
    NSMenu { menu: MenuState },
    /// A popover anchored to the parent view; children are its content.
    NSPopover {
        presented: bool,
        edge: PopoverEdge,
        size: Option<Size>,
    },
    /// An alert sheet on the containing window.
    NSAlert { presented: bool, alert: AlertState },
    NSOpenPanel {
        presented: bool,
        panel: FileDialogState,
    },
    NSSavePanel {
        presented: bool,
        panel: FileDialogState,
    },
    NSColorPanel {
        presented: bool,
        color: Color,
        shows_alpha: bool,
    },
    /// The application's main menu; the first menu is the App menu.
    NSMainMenu { menus: Vec<MenuState> },
}

impl RenderNodeTrait for RenderNode {
    type Event = Event;
    type Context<'a> = crate::AppKitContext<'a>;

    fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[cfg(target_os = "macos")]
impl zintl_ui_appkit_backend::AppKitRenderNode for RenderNode {
    fn create_context<'a>(
        store_context: zintl_ui::view::StoreContext<'a>,
        perform_main: Option<&'a zintl_ui_appkit_backend::MainTaskSender>,
    ) -> Self::Context<'a> {
        crate::AppKitContext::new(store_context, perform_main)
    }

    fn appkit_node(&self) -> zintl_ui_appkit_backend::NodeKind {
        use zintl_ui_appkit_backend::{NodeKind, ViewKind};

        let view = |kind: ViewKind, layout: &LayoutStyle, id: &Option<String>, enabled: bool| {
            NodeKind::View {
                kind,
                layout: *layout,
                id: id.clone(),
                enabled,
            }
        };
        match self {
            Self::NSWindow {
                sidebar,
                bounds,
                title,
                full_size_content_view,
                id,
                style,
            } => NodeKind::Window {
                sidebar: sidebar.as_ref().map(SidebarState::backend),
                bounds: zintl_ui_appkit_backend::Rect::new(
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                ),
                title: title.clone(),
                full_size_content_view: *full_size_content_view,
                id: id.clone(),
                style: style.backend(),
            },
            Self::NSView { layout, id } => view(ViewKind::Container, layout, id, true),
            Self::NSButton {
                title,
                button_type,
                bezel,
                on,
                system_image,
                key_equivalent,
                destructive,
                enabled,
                layout,
                id,
            } => view(
                ViewKind::Button(zintl_ui_appkit_backend::Button {
                    title: title.clone(),
                    button_type: button_type.backend(),
                    bezel: bezel.map(BezelStyle::backend),
                    on: *on,
                    system_image: system_image.clone(),
                    key_equivalent: key_equivalent.clone(),
                    destructive: *destructive,
                }),
                layout,
                id,
                *enabled,
            ),
            Self::NSTextField {
                value,
                placeholder,
                editable,
                selectable,
                bordered,
                draws_background,
                multiline,
                layout,
                id,
            } => view(
                ViewKind::TextField {
                    value: value.clone(),
                    placeholder: placeholder.clone(),
                    editable: *editable,
                    selectable: *selectable,
                    bordered: *bordered,
                    draws_background: *draws_background,
                    multiline: *multiline,
                },
                layout,
                id,
                true,
            ),
            Self::NSSwitch {
                on,
                enabled,
                layout,
                id,
            } => view(ViewKind::Switch { on: *on }, layout, id, *enabled),
            Self::NSSlider {
                value,
                minimum,
                maximum,
                tick_marks,
                snaps_to_ticks,
                enabled,
                layout,
                id,
            } => view(
                ViewKind::Slider {
                    value: *value,
                    minimum: *minimum,
                    maximum: *maximum,
                    tick_marks: *tick_marks,
                    snaps_to_ticks: *snaps_to_ticks,
                },
                layout,
                id,
                *enabled,
            ),
            Self::NSPopUpButton {
                items,
                selected,
                enabled,
                layout,
                id,
            } => view(
                ViewKind::PopUpButton {
                    items: items.clone(),
                    selected: *selected,
                },
                layout,
                id,
                *enabled,
            ),
            Self::NSSegmentedControl {
                segments,
                selected,
                enabled,
                layout,
                id,
            } => view(
                ViewKind::SegmentedControl {
                    segments: segments.clone(),
                    selected: *selected,
                },
                layout,
                id,
                *enabled,
            ),
            Self::NSComboBox {
                items,
                value,
                placeholder,
                enabled,
                layout,
                id,
            } => view(
                ViewKind::ComboBox {
                    items: items.clone(),
                    value: value.clone(),
                    placeholder: placeholder.clone(),
                },
                layout,
                id,
                *enabled,
            ),
            Self::NSNumberField {
                value,
                minimum,
                maximum,
                step,
                fraction_digits,
                enabled,
                layout,
                id,
            } => view(
                ViewKind::NumberField {
                    value: *value,
                    format: zintl_ui_appkit_backend::NumberFormat {
                        minimum: *minimum,
                        maximum: *maximum,
                        step: *step,
                        fraction_digits: *fraction_digits,
                    },
                },
                layout,
                id,
                *enabled,
            ),
            Self::NSSearchField {
                value,
                placeholder,
                layout,
                id,
            } => view(
                ViewKind::SearchField {
                    value: value.clone(),
                    placeholder: placeholder.clone(),
                },
                layout,
                id,
                true,
            ),
            Self::NSColorWell {
                color,
                enabled,
                layout,
                id,
            } => view(
                ViewKind::ColorWell {
                    color: color.backend(),
                },
                layout,
                id,
                *enabled,
            ),
            Self::NSScrollView { axes, layout, id } => view(
                ViewKind::ScrollView {
                    axes: axes.backend(),
                },
                layout,
                id,
                true,
            ),
            Self::NSTabView {
                titles,
                selected,
                layout,
                id,
            } => view(
                ViewKind::TabView {
                    titles: titles.clone(),
                    selected: *selected,
                },
                layout,
                id,
                true,
            ),
            Self::NSMenu { menu } => NodeKind::ContextMenu {
                items: MenuItemState::backend_items(&menu.items),
            },
            Self::NSPopover {
                presented,
                edge,
                size,
            } => NodeKind::Popover {
                presented: *presented,
                edge: edge.backend(),
                size: *size,
            },
            Self::NSAlert { presented, alert } => NodeKind::Alert {
                presented: *presented,
                config: alert.backend(),
            },
            Self::NSOpenPanel { presented, panel } => NodeKind::FileDialog {
                presented: *presented,
                config: panel.backend(zintl_ui_appkit_backend::FileDialogMode::Open),
            },
            Self::NSSavePanel { presented, panel } => NodeKind::FileDialog {
                presented: *presented,
                config: panel.backend(zintl_ui_appkit_backend::FileDialogMode::Save),
            },
            Self::NSColorPanel {
                presented,
                color,
                shows_alpha,
            } => NodeKind::ColorPanel {
                presented: *presented,
                color: color.backend(),
                shows_alpha: *shows_alpha,
            },
            Self::NSMainMenu { menus } => NodeKind::MainMenu(MenuState::backend_menu_bar(menus)),
        }
    }

    fn appkit_event(event: zintl_ui_appkit_backend::AppKitEvent) -> Self::Event {
        use zintl_ui_appkit_backend::AppKitEvent;

        match event {
            AppKitEvent::SidebarSelectionChanged { id } => Event::SidebarSelectionChanged { id },
            AppKitEvent::SidebarSearchChanged { value } => Event::SidebarSearchChanged { value },
            AppKitEvent::Created => Event::WindowCreated,
            AppKitEvent::DidResize => Event::WindowDidResize,
            AppKitEvent::WillClose => Event::WindowWillClose,
            AppKitEvent::DidClose => Event::WindowDidClose,
            AppKitEvent::ButtonClicked => Event::Activated,
            AppKitEvent::TextChanged { value } => Event::TextChanged { value },
            AppKitEvent::StateChanged { on } => Event::StateChanged { on },
            AppKitEvent::ValueChanged { value } => Event::ValueChanged { value },
            AppKitEvent::SelectionChanged { index } => Event::SelectionChanged { index },
            AppKitEvent::ColorChanged { color } => Event::ColorChanged {
                color: Color::from_backend(color),
            },
            AppKitEvent::MenuItemSelected { id } => Event::MenuItemSelected { id },
            AppKitEvent::PopoverClosed => Event::PopoverClosed,
            AppKitEvent::AlertResponse { button } => Event::AlertResponse { button },
            AppKitEvent::FileDialogCompleted { paths } => Event::FileDialogCompleted {
                paths: paths.map(|paths| paths.into_iter().map(PathBuf::from).collect()),
            },
            AppKitEvent::ColorPanelClosed => Event::ColorPanelClosed,
        }
    }

    fn default_main_menu() -> Option<zintl_ui_appkit_backend::CommandSet> {
        let menus = crate::MainMenu::standard().resolve(&mut Default::default());
        Some(MenuState::backend_menu_bar(&menus))
    }
}

#[cfg(target_os = "macos")]
impl AlertState {
    fn backend(&self) -> zintl_ui_appkit_backend::AlertConfig {
        use zintl_ui_appkit_backend::{AlertButton, AlertButtonRole};

        zintl_ui_appkit_backend::AlertConfig {
            style: self.style.backend(),
            title: self.title.clone(),
            message: self.message.clone(),
            buttons: self
                .buttons
                .iter()
                .map(|(title, role)| AlertButton {
                    title: title.clone(),
                    role: match role {
                        crate::ButtonRole::Normal | crate::ButtonRole::Default => {
                            AlertButtonRole::Normal
                        }
                        crate::ButtonRole::Cancel => AlertButtonRole::Cancel,
                        crate::ButtonRole::Destructive => AlertButtonRole::Destructive,
                    },
                })
                .collect(),
        }
    }
}

#[cfg(target_os = "macos")]
impl FileDialogState {
    fn backend(
        &self,
        mode: zintl_ui_appkit_backend::FileDialogMode,
    ) -> zintl_ui_appkit_backend::FileDialogConfig {
        zintl_ui_appkit_backend::FileDialogConfig {
            mode,
            title: self.title.clone(),
            message: self.message.clone(),
            prompt: self.prompt.clone(),
            allowed_extensions: self.allowed_extensions.clone(),
            can_choose_files: self.can_choose_files,
            can_choose_directories: self.can_choose_directories,
            allows_multiple: self.allows_multiple,
            can_create_directories: self.can_create_directories,
            file_name: self.file_name.clone(),
            directory: self
                .directory
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        }
    }
}
