use std::rc::Rc;

use zintl_ui_backend_core::{Color, ViewKind};
use zintl_ui_layout::{LayoutStyle, Size};

use crate::modifiers;
use crate::{
    Attach, BezelStyle, ButtonRole, ButtonType, ChangeRoute, Context, Element, Event, EventKind,
    Menu, NativeView, RenderNode, TextEstimate, ViewModifiers,
};

type Action = dyn for<'a> Fn(&mut Context<'a>);

/// The bezel drawn around a [`Button`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonStyle {
    /// The standard rounded push button.
    #[default]
    Push,
    /// A push button that grows to fit multi-line titles.
    Flexible,
    /// Only the title or symbol.
    Borderless,
    Toolbar,
    /// A compact button for accessory bars.
    Accessory,
    Circular,
    /// A disclosure triangle in a push button.
    Disclosure,
}

#[derive(Clone)]
pub struct Button {
    title: String,
    button_type: ButtonType,
    bezel: Option<BezelStyle>,
    role: ButtonRole,
    system_image: Option<String>,
    on: bool,
    state: ChangeRoute<bool>,
    layout: LayoutStyle,
    modifiers: ViewModifiers,
    action: Option<Rc<Action>>,
    shows_menu: bool,
    keeps_state: bool,
    title_color: Option<Color>,
    /// Created without a title, so the backend measures its size.
    untitled: bool,
}

impl Button {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            button_type: ButtonType::MomentaryPushIn,
            bezel: Some(BezelStyle::Push),
            role: ButtonRole::Normal,
            system_image: None,
            on: false,
            state: ChangeRoute::default(),
            layout: LayoutStyle::leaf(Size::new(80.0, 32.0)),
            modifiers: ViewModifiers::default(),
            action: None,
            shows_menu: false,
            keeps_state: false,
            title_color: None,
            untitled: false,
        }
    }

    /// Creates a button that shows only the SF Symbol `system_image`.
    ///
    /// It takes the symbol's size instead of the standard push button size,
    /// unless a minimum size is declared. Name it with
    /// [`Attach::accessibility_label`], since it has no title for assistive
    /// apps to read.
    pub fn icon(system_image: impl Into<String>) -> Self {
        Self::untitled().system_image(system_image)
    }

    /// Creates the round question mark button that opens help.
    pub fn help() -> Self {
        Self::untitled().bezel_style(Some(BezelStyle::Help))
    }

    /// Creates a button without a title, which the backend measures.
    pub(crate) fn untitled() -> Self {
        Self {
            untitled: true,
            ..Self::new("")
        }
    }

    /// Creates a checkbox whose title is drawn beside the box.
    pub(crate) fn checkbox(title: impl Into<String>) -> Self {
        let title = title.into();
        let width = TextEstimate::width(&title, &TextEstimate::system_font()) + 24.0;
        let mut button = Self::new(title).button_type(ButtonType::Checkbox);
        button.layout.minimum_size = Some(Size::new(width, 20.0));
        button
    }

    /// Creates a radio button; group radios by binding them to one selection.
    pub(crate) fn radio(title: impl Into<String>) -> Self {
        let title = title.into();
        let width = TextEstimate::width(&title, &TextEstimate::system_font()) + 24.0;
        let mut button = Self::new(title).button_type(ButtonType::Radio);
        button.layout.minimum_size = Some(Size::new(width, 20.0));
        button
    }

    pub(crate) fn button_type(mut self, button_type: ButtonType) -> Self {
        self.button_type = button_type;
        self
    }

    /// Sets the bezel; `None` draws a borderless button.
    pub(crate) fn bezel_style(mut self, bezel: Option<BezelStyle>) -> Self {
        self.bezel = bezel;
        self
    }

    /// Selects the bezel drawn around the button.
    pub fn style(self, style: ButtonStyle) -> Self {
        self.bezel_style(match style {
            ButtonStyle::Push => Some(BezelStyle::Push),
            ButtonStyle::Flexible => Some(BezelStyle::FlexiblePush),
            ButtonStyle::Borderless => None,
            ButtonStyle::Toolbar => Some(BezelStyle::Toolbar),
            ButtonStyle::Accessory => Some(BezelStyle::AccessoryBarAction),
            ButtonStyle::Circular => Some(BezelStyle::Circular),
            ButtonStyle::Disclosure => Some(BezelStyle::PushDisclosure),
        })
    }

    /// `Default` answers Return, `Cancel` answers Escape, and `Destructive` warns.
    pub fn role(mut self, role: ButtonRole) -> Self {
        self.role = role;
        self
    }

    /// Draws the title in `color` instead of the platform's color.
    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = Some(color);
        self
    }

    /// Shows an SF Symbol beside the title; [`Button::icon`] shows it alone.
    pub fn system_image(mut self, name: impl Into<String>) -> Self {
        self.system_image = Some(name.into());
        self
    }

    /// Sets the initial state of a toggle, checkbox, or radio button.
    pub(crate) fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Reads the on state of a toggle, checkbox, or radio button from the
    /// route's Store and reports changes to it.
    pub(crate) fn route_state(mut self, route: ChangeRoute<bool>) -> Self {
        self.state = route;
        self
    }

    /// Shows `menu` below the button when clicked, like a pull-down button.
    ///
    /// The button then runs the chosen item's action instead of `on_click`.
    pub fn menu(mut self, menu: impl Into<Menu>) -> Self {
        self.shows_menu = true;
        self.context_menu(menu)
    }

    /// Shows the button as selected, such as the chosen row number in a list.
    ///
    /// It draws in the on state; clicks report `on_click` and leave the
    /// state to the next render.
    pub fn selected(mut self, selected: bool) -> Self {
        self.button_type = ButtonType::PushOnPushOff;
        self.on = selected;
        self.keeps_state = true;
        self
    }

    pub fn on_click(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }
}

impl Button {
    /// Returns the layout; untitled buttons leave their size to the backend's measuring.
    fn layout(&self) -> LayoutStyle {
        let mut layout = self.layout;
        if self.untitled && self.modifiers.layout().minimum_size.is_none() {
            layout.minimum_size = Some(Size::default());
        }
        layout
    }
}

impl NativeView for Button {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let on = self.state.store().map_or(self.on, |store| *cx.get(store));
        let key_equivalent = match self.role {
            ButtonRole::Default => Some("\r".to_owned()),
            ButtonRole::Cancel => Some("\u{1b}".to_owned()),
            ButtonRole::Normal | ButtonRole::Destructive => None,
        };
        let mut element = Element::node(RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                title: self.title.clone(),
                button_type: self.button_type,
                bezel: self.bezel,
                on,
                system_image: self.system_image.clone(),
                key_equivalent,
                destructive: self.role == ButtonRole::Destructive,
                shows_menu: self.shows_menu,
                keeps_state: self.keeps_state,
                title_color: self.title_color,
            }),
            layout: self.layout(),
            id: None,
            enabled: true,
            title_bar_interactive: false,
            help: None,
            accessibility: Default::default(),
        });
        if let Some(action) = self.action.clone() {
            element = element.on_event(EventKind::Activated, move |cx, event| {
                if event == Event::Activated {
                    action(cx);
                }
            });
        }
        self.state
            .clone()
            .route(element, EventKind::StateChanged, |event| match event {
                Event::StateChanged { on } => Some(on),
                _ => None,
            })
    }
}

native_view!(Button);
