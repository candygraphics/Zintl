use std::rc::Rc;

use zintl_ui::element::IntoElement;
use zintl_ui::store::Store;
use zintl_ui::view::View;
use zintl_ui_backend_core::ViewKind;
use zintl_ui_layout::{LayoutStyle, Size};

use crate::attach::attachments;
use crate::{
    Attachments, BezelStyle, ButtonRole, ButtonType, Context, Element, Event, EventKind,
    NativeView, RenderNode,
};

type Action = dyn for<'a> Fn(&mut Context<'a>);
type StateAction = dyn for<'a> Fn(&mut Context<'a>, bool);

#[derive(Clone)]
pub struct Button {
    title: String,
    button_type: ButtonType,
    bezel: Option<BezelStyle>,
    role: ButtonRole,
    system_image: Option<String>,
    on: bool,
    state: Option<Store<bool>>,
    enabled: bool,
    layout: LayoutStyle,
    id: Option<String>,
    attachments: Attachments,
    action: Option<Rc<Action>>,
    state_action: Option<Rc<StateAction>>,
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
            state: None,
            enabled: true,
            layout: LayoutStyle::leaf(Size::new(80.0, 32.0)),
            id: None,
            attachments: Attachments::default(),
            action: None,
            state_action: None,
        }
    }

    /// Creates a checkbox whose title is drawn beside the box.
    pub fn checkbox(title: impl Into<String>) -> Self {
        let title = title.into();
        let width = title.chars().count() as f32 * 7.0 + 24.0;
        Self::new(title)
            .button_type(ButtonType::Checkbox)
            .minimum_size(Size::new(width, 20.0))
    }

    /// Creates a radio button; group radios by binding them to one selection.
    pub fn radio(title: impl Into<String>) -> Self {
        let title = title.into();
        let width = title.chars().count() as f32 * 7.0 + 24.0;
        Self::new(title)
            .button_type(ButtonType::Radio)
            .minimum_size(Size::new(width, 20.0))
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn minimum_size(mut self, size: Size) -> Self {
        self.layout.minimum_size = size;
        self
    }

    pub fn button_type(mut self, button_type: ButtonType) -> Self {
        self.button_type = button_type;
        self
    }

    /// Sets the bezel; `None` draws a borderless button.
    pub fn bezel_style(mut self, bezel: Option<BezelStyle>) -> Self {
        self.bezel = bezel;
        self
    }

    pub fn role(mut self, role: ButtonRole) -> Self {
        self.role = role;
        self
    }

    /// Shows an SF Symbol; an empty title shows the symbol alone.
    pub fn system_image(mut self, name: impl Into<String>) -> Self {
        self.system_image = Some(name.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the initial state of a toggle, checkbox, or radio button.
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Reads and writes the on state of a toggle, checkbox, or radio button.
    pub fn bind_state(mut self, store: Store<bool>) -> Self {
        self.state = Some(store);
        self
    }

    /// Schedules `action` when the user activates this button.
    pub fn on_click(mut self, action: impl for<'a> Fn(&mut Context<'a>) + 'static) -> Self {
        self.action = Some(Rc::new(action));
        self
    }

    /// Runs after a toggle, checkbox, or radio button changes state.
    pub fn on_state_change(
        mut self,
        action: impl for<'a> Fn(&mut Context<'a>, bool) + 'static,
    ) -> Self {
        self.state_action = Some(Rc::new(action));
        self
    }
}

impl NativeView for Button {
    attachments!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let on = self.state.map_or(self.on, |store| *cx.get(store));
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
            }),
            layout: self.layout,
            id: self.id.clone(),
            enabled: self.enabled,
        });
        if let Some(action) = self.action.clone() {
            element = element.on_event(EventKind::Activated, move |cx, event| {
                if event == Event::Activated {
                    action(cx);
                }
            });
        }
        if self.state.is_some() || self.state_action.is_some() {
            let store = self.state;
            let action = self.state_action.clone();
            element = element.on_event(EventKind::StateChanged, move |cx, event| {
                if let Event::StateChanged { on } = event {
                    if let Some(store) = store {
                        cx.update(store, |current| *current = on);
                    }
                    if let Some(action) = &action {
                        action(cx, on);
                    }
                }
            });
        }
        element
    }
}

impl View for Button {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = Self::Output> {
        self.native_element(cx)
    }
}
