//! Popovers, sheets, and panels presented while a Store is `true`.

use std::path::PathBuf;
use std::rc::Rc;

use zintl_ui::element::{ElementFactory, IntoElement};
use zintl_ui::store::Store;
use zintl_ui_layout::Size;

use zintl_ui_backend_core::{AlertButtonRole, AlertConfig, FileDialogConfig, FileDialogMode};

use crate::{
    AlertStyle, ButtonRole, Color, Context, Element, Event, EventKind, PopoverEdge, RenderNode,
};

type Action = dyn for<'a> Fn(&mut Context<'a>);
type PathsAction = dyn for<'a> Fn(&mut Context<'a>, &[PathBuf]);
type ColorAction = dyn for<'a> Fn(&mut Context<'a>, Color);

/// Reads a presentation flag; unbound presentations stay hidden.
struct Presented(Option<Store<bool>>);

impl Presented {
    fn get(&self, cx: &Context<'_>) -> bool {
        self.0.is_some_and(|store| *cx.get(store))
    }

    /// Clears the bound flag after the platform dismissed the presentation.
    fn dismissed(store: Option<Store<bool>>, cx: &mut Context<'_>) {
        if let Some(store) = store {
            cx.update(store, |presented| *presented = false);
        }
    }
}

/// A transient popover shown while its bound Store is `true`.
#[derive(Clone)]
pub struct Popover {
    content: ElementFactory<RenderNode>,
    presented: Option<Store<bool>>,
    edge: PopoverEdge,
    size: Option<Size>,
    on_close: Option<Rc<Action>>,
}

impl Popover {
    pub fn new<V>(content: V) -> Self
    where
        V: Clone + IntoElement<Output = RenderNode> + 'static,
    {
        Self {
            content: ElementFactory::new(content),
            presented: None,
            edge: PopoverEdge::Bottom,
            size: None,
            on_close: None,
        }
    }

    /// Shows the popover while `presented` is `true`; closing it writes `false`.
    pub fn bind(mut self, presented: Store<bool>) -> Self {
        self.presented = Some(presented);
        self
    }

    pub fn edge(mut self, edge: PopoverEdge) -> Self {
        self.edge = edge;
        self
    }

    /// Fixes the content size; otherwise the popover fits its content.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    pub fn on_close(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.on_close = Some(Rc::new(action));
        self
    }

    pub(crate) fn element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let store = self.presented;
        let on_close = self.on_close.clone();
        Element::node(RenderNode::Popover {
            presented: Presented(store).get(cx),
            edge: self.edge,
            size: self.size,
        })
        .with_children([self.content.clone().into_element()])
        .on_event(EventKind::PopoverClosed, move |cx, _| {
            Presented::dismissed(store, cx);
            if let Some(action) = &on_close {
                action(cx);
            }
        })
    }
}

/// A button of an [`Alert`].
#[derive(Clone)]
pub struct AlertButton {
    title: String,
    role: ButtonRole,
    action: Option<Rc<Action>>,
}

impl AlertButton {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            role: ButtonRole::Normal,
            action: None,
        }
    }

    /// `Cancel` answers Escape and `Destructive` draws the button as destructive.
    pub fn role(mut self, role: ButtonRole) -> Self {
        self.role = role;
        self
    }

    pub fn on_click(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

/// An alert shown on its window while its bound Store is `true`.
///
/// The first button answers Return. An OK button is added when none is given.
#[derive(Clone)]
pub struct Alert {
    title: String,
    message: Option<String>,
    style: AlertStyle,
    buttons: Vec<AlertButton>,
    presented: Option<Store<bool>>,
}

impl Alert {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: None,
            style: AlertStyle::Warning,
            buttons: Vec::new(),
            presented: None,
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn style(mut self, style: AlertStyle) -> Self {
        self.style = style;
        self
    }

    /// Adds a button; buttons are laid out from the trailing edge.
    pub fn button(mut self, button: AlertButton) -> Self {
        self.buttons.push(button);
        self
    }

    pub fn bind(mut self, presented: Store<bool>) -> Self {
        self.presented = Some(presented);
        self
    }

    pub(crate) fn element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let store = self.presented;
        let actions: Vec<_> = self
            .buttons
            .iter()
            .map(|button| button.action.clone())
            .collect();
        Element::node(RenderNode::Alert {
            presented: Presented(store).get(cx),
            config: AlertConfig {
                style: self.style,
                title: self.title.clone(),
                message: self.message.clone(),
                buttons: self
                    .buttons
                    .iter()
                    .map(|button| zintl_ui_backend_core::AlertButton {
                        title: button.title.clone(),
                        role: match button.role {
                            ButtonRole::Normal | ButtonRole::Default => AlertButtonRole::Normal,
                            ButtonRole::Cancel => AlertButtonRole::Cancel,
                            ButtonRole::Destructive => AlertButtonRole::Destructive,
                        },
                    })
                    .collect(),
            },
        })
        .on_event(EventKind::AlertResponse, move |cx, event| {
            if let Event::AlertResponse { button } = event {
                Presented::dismissed(store, cx);
                if let Some(Some(action)) = button.and_then(|index| actions.get(index)) {
                    action(cx);
                }
            }
        })
    }
}

/// An open or save dialog shown while its bound Store is `true`.
#[derive(Clone)]
pub struct FileDialog {
    config: FileDialogConfig,
    presented: Option<Store<bool>>,
    on_select: Option<Rc<PathsAction>>,
    on_cancel: Option<Rc<Action>>,
}

impl FileDialog {
    /// Chooses existing files.
    pub fn open() -> Self {
        Self {
            config: FileDialogConfig {
                mode: FileDialogMode::Open,
                can_choose_files: true,
                ..FileDialogConfig::default()
            },
            presented: None,
            on_select: None,
            on_cancel: None,
        }
    }

    /// Chooses a destination for a new file.
    pub fn save() -> Self {
        Self {
            config: FileDialogConfig {
                mode: FileDialogMode::Save,
                can_create_directories: true,
                ..FileDialogConfig::default()
            },
            ..Self::open()
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = Some(title.into());
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.config.message = Some(message.into());
        self
    }

    /// Sets the confirmation button title.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.prompt = Some(prompt.into());
        self
    }

    /// Limits choices to these filename extensions, written without dots.
    pub fn allowed_extensions(
        mut self,
        extensions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.config.allowed_extensions = extensions.into_iter().map(Into::into).collect();
        self
    }

    pub fn choose_files(mut self, enabled: bool) -> Self {
        self.config.can_choose_files = enabled;
        self
    }

    pub fn choose_directories(mut self, enabled: bool) -> Self {
        self.config.can_choose_directories = enabled;
        self
    }

    pub fn multiple(mut self) -> Self {
        self.config.allows_multiple = true;
        self
    }

    pub fn can_create_directories(mut self, enabled: bool) -> Self {
        self.config.can_create_directories = enabled;
        self
    }

    /// Prefills the save panel's name field.
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.config.file_name = Some(name.into());
        self
    }

    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.config.directory = Some(directory.into().to_string_lossy().into_owned());
        self
    }

    pub fn bind(mut self, presented: Store<bool>) -> Self {
        self.presented = Some(presented);
        self
    }

    /// Receives the chosen paths; a save panel reports one path.
    pub fn on_select(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, &[PathBuf]) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(action));
        self
    }

    pub fn on_cancel(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(action));
        self
    }

    pub(crate) fn element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let store = self.presented;
        let presented = Presented(store).get(cx);
        let node = RenderNode::FileDialog {
            presented,
            config: self.config.clone(),
        };
        let on_select = self.on_select.clone();
        let on_cancel = self.on_cancel.clone();
        Element::node(node).on_event(EventKind::FileDialogCompleted, move |cx, event| {
            if let Event::FileDialogCompleted { paths } = event {
                Presented::dismissed(store, cx);
                match (paths, &on_select, &on_cancel) {
                    (Some(paths), Some(action), _) => action(cx, &paths),
                    (None, _, Some(action)) => action(cx),
                    _ => {}
                }
            }
        })
    }
}

/// Connects the shared color panel to a window while its bound Store is `true`.
#[derive(Clone)]
pub struct ColorPanel {
    color: Option<Store<Color>>,
    presented: Option<Store<bool>>,
    shows_alpha: bool,
    on_change: Option<Rc<ColorAction>>,
}

impl ColorPanel {
    pub fn new() -> Self {
        Self {
            color: None,
            presented: None,
            shows_alpha: false,
            on_change: None,
        }
    }

    /// Reads and writes the edited color.
    pub fn bind_color(mut self, color: Store<Color>) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bind(mut self, presented: Store<bool>) -> Self {
        self.presented = Some(presented);
        self
    }

    pub fn shows_alpha(mut self, shows_alpha: bool) -> Self {
        self.shows_alpha = shows_alpha;
        self
    }

    pub fn on_change(mut self, action: impl for<'a> Fn(&mut Context<'a>, Color) + 'static) -> Self {
        self.on_change = Some(Rc::new(action));
        self
    }

    pub(crate) fn element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let store = self.presented;
        let color_store = self.color;
        let on_change = self.on_change.clone();
        Element::node(RenderNode::ColorPanel {
            presented: Presented(store).get(cx),
            color: color_store.map_or(Color::WHITE, |color| *cx.get(color)),
            shows_alpha: self.shows_alpha,
        })
        .on_event(EventKind::ColorChanged, move |cx, event| {
            if let Event::ColorChanged { color } = event {
                if let Some(store) = color_store {
                    cx.update(store, |current| *current = color);
                }
                if let Some(action) = &on_change {
                    action(cx, color);
                }
            }
        })
        .on_event(EventKind::ColorPanelClosed, move |cx, _| {
            Presented::dismissed(store, cx);
        })
    }
}

impl Default for ColorPanel {
    fn default() -> Self {
        Self::new()
    }
}
