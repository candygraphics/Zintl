use zintl_ui_backend_core::{Accessibility, AccessibilityRole};
use zintl_ui_layout::{LayoutDimension, LayoutStyle, Size};

use crate::{Context, Element, Menu, Popover, RenderNode, ViewId};

/// A view that renders exactly one native element.
///
/// Built-in views implement this so the shared modifiers can reach the
/// native view they render. A view composed of other views implements it
/// too when its root is one native view node, such as a stack:
///
/// ```
/// use zintl_ui_native::*;
///
/// #[derive(Clone)]
/// pub struct Card {
///     title: String,
///     modifiers: ViewModifiers,
/// }
///
/// impl NativeView for Card {
///     modifiers!();
///
///     fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
///         VStack::new(list![Text::new(self.title.clone())])
///             .padding(12.0)
///             .native_element(cx)
///     }
/// }
///
/// native_view!(Card);
///
/// let card = Card { title: "Inbox".into(), modifiers: ViewModifiers::default() }
///     .id("inbox")
///     .width(200.0);
/// ```
///
/// # Layering
///
/// Modifiers on the outer view win over those its root declares:
///
/// - `id`, `enabled`, sizes, and `help` replace the root's when declared.
/// - Accessibility replaces only the properties the outer view declares.
/// - `context_menu` replaces the root's menu.
/// - Popovers of both stay, the root's first.
///
/// # Conditions
///
/// - `base_element` must return a native view node. A node from
///   `cx.watch` or another `Element::Bound` cannot take the root modifiers,
///   which debug builds report.
/// - A view with `View::init`, such as [`crate::SplitView`] or
///   [`crate::DisclosureGroup`], only initializes when mounted as a child,
///   through `list!` or `content(..)`. Do not call its `native_element`
///   from another view's `base_element`.
pub trait NativeView: Clone + 'static {
    /// Renders the native element without its modifiers.
    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode>;

    fn modifiers(&self) -> &ViewModifiers;

    fn modifiers_mut(&mut self) -> &mut ViewModifiers;

    /// Returns whether the view hands its ID to an inner control itself,
    /// such as a labeled switch whose root is only a row.
    fn places_id(&self) -> bool {
        false
    }

    /// Renders the native element with its modifiers applied.
    fn native_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        let mut element = self.base_element(cx);
        let modifiers = self.modifiers();
        if let Element::Node {
            value:
                RenderNode::View {
                    layout,
                    id,
                    enabled,
                    title_bar_interactive,
                    help,
                    accessibility,
                    ..
                },
            ..
        } = &mut element
        {
            if !self.places_id()
                && let Some(value) = &modifiers.id
            {
                *id = Some(value.as_str().to_owned());
            }
            if let Some(value) = modifiers.enabled {
                *enabled = value;
            }
            modifiers.layout.apply(layout);
            *title_bar_interactive |= modifiers.title_bar_interactive;
            if modifiers.help.is_some() {
                help.clone_from(&modifiers.help);
            }
            // Views may describe themselves; modifiers win over that.
            accessibility.merge(&modifiers.accessibility);
        } else {
            debug_assert!(
                !modifiers.targets_root(),
                "layout, ID, and enabled modifiers need a view whose root is a native view node"
            );
        }
        modifiers.attach(element, cx)
    }
}

/// Size modifiers available on every [`NativeView`].
///
/// They override the view's own layout, whatever order they are called in.
pub trait Layout: NativeView + Sized {
    /// Fixes the width in points.
    fn width(mut self, width: f32) -> Self {
        self.modifiers_mut().layout.width = Some(LayoutDimension::Points(width));
        self
    }

    /// Fixes the height in points.
    ///
    /// A view that fills the remaining space by default, such as a scroll
    /// view, then stops growing unless [`Layout::grow`] is also called.
    fn height(mut self, height: f32) -> Self {
        self.modifiers_mut().layout.height = Some(LayoutDimension::Points(height));
        self
    }

    /// Fixes the width and height in points.
    fn size(self, size: Size) -> Self {
        self.width(size.width).height(size.height)
    }

    /// Takes the parent's full width.
    fn fill_width(mut self) -> Self {
        self.modifiers_mut().layout.width = Some(LayoutDimension::Percent(1.0));
        self
    }

    /// Takes the parent's full height.
    fn fill_height(mut self) -> Self {
        self.modifiers_mut().layout.height = Some(LayoutDimension::Percent(1.0));
        self
    }

    /// Keeps the view at least `size` large.
    fn minimum_size(mut self, size: Size) -> Self {
        self.modifiers_mut().layout.minimum_size = Some(size);
        self
    }

    /// Takes the remaining space along the parent's main axis.
    fn grow(mut self) -> Self {
        self.modifiers_mut().layout.grow = Some(1.0);
        self
    }
}

impl<V: NativeView> Layout for V {}

/// The enabled state, available on every [`NativeView`].
///
/// # Values
///
/// Controls that edit a value share one vocabulary:
///
/// - `value(..)`, or `selection(..)` for a choice, shows a value the app owns.
/// - `bind(store)` reads and writes the value in a Store; it takes precedence
///   over `value`. Other Stores have named binders, such as `bind_presented`.
/// - `on_change` runs after each change, once the bound Store is updated.
///   `Copy` values arrive by value and text as `&str`.
///
/// Without `bind`, a control follows `value` in one of two ways:
///
/// - Discrete controls ([`crate::Toggle`], [`crate::Picker`],
///   [`crate::TabView`], [`crate::Slider`], [`crate::ColorWell`],
///   [`crate::DatePicker`]) always show `value`: a change that `on_change`
///   does not turn into a new value is undone.
/// - Text inputs ([`crate::TextField`], [`crate::NumberField`],
///   [`crate::IntegerField`], [`crate::ComboBox`], [`crate::SearchField`])
///   keep what the user typed until `value` itself changes.
pub trait Control: NativeView + Sized {
    /// Disables the view and every view inside it with `false`; they then
    /// ignore input and draw dimmed.
    ///
    /// A view inside a disabled one stays disabled whatever it declares.
    fn enabled(mut self, enabled: bool) -> Self {
        self.modifiers_mut().enabled = Some(enabled);
        self
    }
}

impl<V: NativeView> Control for V {}

/// Presentation modifiers available on every [`NativeView`].
///
/// They return `Self`, so the view's own builder methods can follow.
pub trait Attach: NativeView + Sized {
    /// Names the view for accessibility, UI tests, and scroll targets.
    fn id(mut self, id: impl Into<ViewId>) -> Self {
        self.modifiers_mut().id = Some(id.into());
        self
    }

    /// Shows `menu` when the view is secondary-clicked.
    fn context_menu(mut self, menu: impl Into<Menu>) -> Self {
        self.modifiers_mut().menu = Some(menu.into());
        self
    }

    /// Anchors `popover` to this view.
    fn popover(mut self, popover: Popover) -> Self {
        self.modifiers_mut().popovers.push(popover);
        self
    }

    /// Keeps mouse input for this view and its subviews inside the title bar area with `true`.
    ///
    /// When a window's content extends under its title bar, clicks there move
    /// the window unless they land on a view marked with this modifier.
    fn title_bar_interactive(mut self, interactive: bool) -> Self {
        self.modifiers_mut().title_bar_interactive = interactive;
        self
    }

    /// Shows `text` when the pointer rests on the view (`NSView.toolTip`).
    ///
    /// It also shows while the view is disabled, so it can say why.
    fn help(mut self, text: impl Into<String>) -> Self {
        self.modifiers_mut().help = Some(text.into());
        self
    }

    /// Names the view for assistive apps such as VoiceOver.
    ///
    /// Use it for controls without visible text, such as a button showing only
    /// a symbol. A label also turns a container or canvas into a named element.
    fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.modifiers_mut().accessibility.label = Some(label.into());
        self
    }

    /// Describes the view's current value, such as "3 of 10 done".
    fn accessibility_value(mut self, value: impl Into<String>) -> Self {
        self.modifiers_mut().accessibility.value = Some(value.into());
        self
    }

    /// Explains what using the view does, such as "Removes the selected train".
    fn accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.modifiers_mut().accessibility.hint = Some(hint.into());
        self
    }

    /// Replaces the role assistive apps report, such as [`AccessibilityRole::Group`]
    /// for a stack that reads as one section.
    fn accessibility_role(mut self, role: AccessibilityRole) -> Self {
        self.modifiers_mut().accessibility.role = Some(role);
        self
    }

    /// Hides the view itself from assistive apps with `true`, such as
    /// decorative text that repeats a neighbor; its subviews stay reachable.
    fn accessibility_hidden(mut self, hidden: bool) -> Self {
        self.modifiers_mut().accessibility.hidden = hidden;
        self
    }

    /// Names the view with the text of the view whose ID is `id`, such as a
    /// `Text` beside a slider.
    fn accessibility_labeled_by(mut self, id: impl Into<ViewId>) -> Self {
        self.modifiers_mut().accessibility.labeled_by = Some(id.into().into());
        self
    }
}

impl<V: NativeView> Attach for V {}

/// The sizes a view declared with [`Layout`], laid over its own layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutOverrides {
    pub width: Option<LayoutDimension>,
    pub height: Option<LayoutDimension>,
    pub minimum_size: Option<Size>,
    pub grow: Option<f32>,
}

impl LayoutOverrides {
    /// Writes the declared sizes into `layout`.
    pub fn apply(&self, layout: &mut LayoutStyle) {
        if let Some(width) = self.width {
            layout.width = width;
        }
        if let Some(height) = self.height {
            layout.height = height;
            // A fixed height replaces filling the remaining space.
            if matches!(height, LayoutDimension::Points(_)) {
                layout.grow = 0.0;
            }
        }
        if let Some(minimum_size) = self.minimum_size {
            layout.minimum_size = Some(minimum_size);
        }
        if let Some(grow) = self.grow {
            layout.grow = grow;
        }
    }

    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The ID, enabled state, sizes, and presentations declared on a native view.
#[derive(Clone, Default)]
pub struct ViewModifiers {
    id: Option<ViewId>,
    enabled: Option<bool>,
    layout: LayoutOverrides,
    menu: Option<Menu>,
    popovers: Vec<Popover>,
    title_bar_interactive: bool,
    help: Option<String>,
    accessibility: Accessibility,
}

impl ViewModifiers {
    /// Returns the declared ID.
    pub fn id(&self) -> Option<&ViewId> {
        self.id.as_ref()
    }

    /// Returns whether the view is enabled; views are enabled unless declared otherwise.
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    /// Returns the declared sizes.
    pub fn layout(&self) -> &LayoutOverrides {
        &self.layout
    }

    /// Returns whether a modifier that is written onto the root node is declared.
    fn targets_root(&self) -> bool {
        self.id.is_some() || self.enabled.is_some() || !self.layout.is_empty()
    }

    fn attach(
        &self,
        mut element: Element<RenderNode>,
        cx: &mut Context<'_>,
    ) -> Element<RenderNode> {
        if self.menu.is_none() && self.popovers.is_empty() {
            return element;
        }
        let mut attachments = Vec::new();
        if let Some(menu) = &self.menu {
            // An outer view's menu replaces the one its root declared.
            if let Element::Node { children, .. } = &mut element {
                children.retain(|child| {
                    !matches!(
                        child,
                        Element::Node {
                            value: RenderNode::ContextMenu { .. },
                            ..
                        }
                    )
                });
            }
            attachments.push(menu.context_element());
        }
        attachments.extend(self.popovers.iter().map(|popover| popover.element(cx)));
        element.with_children(attachments)
    }
}

/// Implements the [`NativeView`] modifier accessors for a `modifiers` field.
#[macro_export]
macro_rules! modifiers {
    () => {
        fn modifiers(&self) -> &$crate::ViewModifiers {
            &self.modifiers
        }

        fn modifiers_mut(&mut self) -> &mut $crate::ViewModifiers {
            &mut self.modifiers
        }
    };
}

#[cfg(test)]
mod tests {
    use zintl_ui::element::IntoElement;
    use zintl_ui_layout::{Axis, LayoutDimension, LayoutStyle, Size};

    use crate::testing::Harness;
    use crate::{
        Attach, Button, Container, Control, Layout, RenderNode, ScrollView, Slider, Table,
        TableColumn, TableRow, Text, TextField, ViewId,
    };

    /// A mounted view, read back as render nodes.
    struct Mounted(Harness);

    impl Mounted {
        fn new(root: impl IntoElement<Output = RenderNode>) -> Self {
            Self(Harness::new(root))
        }

        /// Returns whether the view at `path` below the root view and every view above it are enabled.
        fn is_enabled(&self, path: &[usize]) -> bool {
            self.0.is_enabled(&[&[0], path].concat())
        }

        /// Returns the layout, ID, and enabled state declared on the node at `path` below the root view.
        fn view(&self, path: &[usize]) -> (LayoutStyle, Option<String>, bool) {
            let mut node = self.0.tree();
            for index in path {
                node = node.children.swap_remove(*index);
            }
            match node.value {
                RenderNode::View {
                    layout,
                    id,
                    enabled,
                    ..
                } => (layout, id, enabled),
                other => panic!("expected a view, got {other:?}"),
            }
        }

        /// Declares every size and an ID on `view` and checks they reach its node.
        fn assert_modifiers_reach<V>(view: V)
        where
            V: Layout + Attach + IntoElement<Output = RenderNode>,
        {
            let mounted = Self::new(
                view.width(120.0)
                    .height(40.0)
                    .minimum_size(Size::new(10.0, 12.0))
                    .grow()
                    .id("target"),
            );
            let (layout, id, _) = mounted.view(&[]);

            assert_eq!(layout.width, LayoutDimension::Points(120.0));
            assert_eq!(layout.height, LayoutDimension::Points(40.0));
            assert_eq!(layout.minimum_size, Some(Size::new(10.0, 12.0)));
            assert_eq!(layout.grow, 1.0);
            assert_eq!(id.as_deref(), Some("target"));
        }

        fn empty() -> Container {
            Container::new(LayoutStyle::stack(Axis::Vertical, 0.0), Vec::new())
        }
    }

    #[test]
    fn size_and_id_modifiers_reach_every_kind_of_view() {
        // Verifies controls, text, and containers share the same size and ID modifiers.
        Mounted::assert_modifiers_reach(Button::new("Save"));
        Mounted::assert_modifiers_reach(TextField::new());
        Mounted::assert_modifiers_reach(Text::new("Label"));
        Mounted::assert_modifiers_reach(Slider::new(0.0..=1.0));
        Mounted::assert_modifiers_reach(ScrollView::new(Mounted::empty()));
        Mounted::assert_modifiers_reach(Mounted::empty());
    }

    #[test]
    fn id_accepts_literals_strings_constants_and_view_ids() {
        // Verifies every way of naming a view reaches its node as the same string.
        const SAVE: &str = "save";
        let shared = ViewId::from(SAVE);
        let ids = [
            Mounted::new(Button::new("Save").id("save")).view(&[]).1,
            Mounted::new(Button::new("Save").id(String::from("save")))
                .view(&[])
                .1,
            Mounted::new(Button::new("Save").id(SAVE)).view(&[]).1,
            Mounted::new(Button::new("Save").id(&shared)).view(&[]).1,
            Mounted::new(Button::new("Save").id(format!("{shared}")))
                .view(&[])
                .1,
        ];

        assert!(ids.iter().all(|id| id.as_deref() == Some("save")));
        assert_eq!(shared.as_str(), "save");
        assert_eq!(String::from(shared), "save");
    }

    #[test]
    fn size_sets_width_and_height_and_fill_takes_the_parent_size() {
        // Verifies `size` fixes both dimensions and the fill modifiers use the parent's full size.
        let (fixed, _, _) = Mounted::new(Button::new("Save").size(Size::new(90.0, 30.0))).view(&[]);
        let (filling, _, _) = Mounted::new(Mounted::empty().fill_width().fill_height()).view(&[]);

        assert_eq!(fixed.width, LayoutDimension::Points(90.0));
        assert_eq!(fixed.height, LayoutDimension::Points(30.0));
        assert_eq!(filling.width, LayoutDimension::Percent(1.0));
        assert_eq!(filling.height, LayoutDimension::Percent(1.0));
    }

    #[test]
    fn fixed_height_stops_a_filling_view_from_growing() {
        // Verifies a fixed height replaces filling the remaining space unless growth is declared.
        let (filling, _, _) = Mounted::new(ScrollView::new(Mounted::empty())).view(&[]);
        let (fixed, _, _) = Mounted::new(ScrollView::new(Mounted::empty()).height(120.0)).view(&[]);
        let (growing, _, _) =
            Mounted::new(ScrollView::new(Mounted::empty()).grow().height(120.0)).view(&[]);

        assert_eq!(filling.grow, 1.0);
        assert_eq!(fixed.height, LayoutDimension::Points(120.0));
        assert_eq!(fixed.grow, 0.0);
        assert_eq!(growing.grow, 1.0);
    }

    #[test]
    fn icon_only_button_keeps_a_declared_minimum_size() {
        // Verifies a symbol-only button is measured by the backend unless the app declares a minimum.
        let (measured, _, _) = Mounted::new(Button::icon("gearshape")).view(&[]);
        let (declared, _, _) =
            Mounted::new(Button::icon("gearshape").minimum_size(Size::new(32.0, 32.0))).view(&[]);

        assert_eq!(measured.minimum_size, Some(Size::default()));
        assert_eq!(declared.minimum_size, Some(Size::new(32.0, 32.0)));
    }

    #[test]
    fn label_given_a_width_wraps_to_it() {
        // Verifies a label that takes its parent's width wraps instead of reserving its text's width.
        let text = "A sentence that is long enough to wrap";
        let (natural, _, _) = Mounted::new(Text::new(text)).view(&[]);
        let (filling, _, _) = Mounted::new(Text::new(text).fill_width()).view(&[]);

        assert!(!natural.wraps);
        assert!(natural.minimum_size.is_some_and(|size| size.width > 0.0));
        assert!(filling.wraps);
        assert_eq!(filling.width, LayoutDimension::Percent(1.0));
        assert!(filling.minimum_size.is_some_and(|size| size.width == 0.0));
    }

    #[test]
    fn disabling_a_container_disables_the_views_inside_it() {
        // Verifies a disabled container disables its descendants, whatever they declare themselves.
        let mounted = Mounted::new(
            Container::new(
                LayoutStyle::stack(Axis::Vertical, 0.0),
                crate::list![
                    Button::new("Save").enabled(true),
                    Container::new(
                        LayoutStyle::stack(Axis::Vertical, 0.0),
                        crate::list![TextField::new()],
                    ),
                ],
            )
            .enabled(false),
        );
        let enabled = Mounted::new(Container::new(
            LayoutStyle::stack(Axis::Vertical, 0.0),
            crate::list![Button::new("Save")],
        ));

        assert!(!mounted.is_enabled(&[]));
        assert!(!mounted.is_enabled(&[0]));
        assert!(!mounted.is_enabled(&[1, 0]));
        // The button's own node still says what it declared.
        assert!(mounted.view(&[0]).2);
        assert!(enabled.is_enabled(&[0]));
    }

    #[test]
    fn controls_are_enabled_until_disabled() {
        // Verifies `enabled(false)` reaches the node, including fields that could not be disabled before.
        let (_, _, button) = Mounted::new(Button::new("Save")).view(&[]);
        let (_, _, disabled_button) = Mounted::new(Button::new("Save").enabled(false)).view(&[]);
        let (_, _, disabled_field) = Mounted::new(TextField::new().enabled(false)).view(&[]);

        assert!(button);
        assert!(!disabled_button);
        assert!(!disabled_field);
    }

    /// A composed view whose root stack declares its own modifiers.
    #[derive(Clone)]
    struct Card {
        modifiers: crate::ViewModifiers,
    }

    impl crate::NativeView for Card {
        crate::modifiers!();

        fn base_element(&self, cx: &mut crate::Context<'_>) -> crate::Element<RenderNode> {
            crate::VStack::new(crate::list![Text::new("Card")])
                .id("inner")
                .width(80.0)
                .accessibility_label("Inner")
                .accessibility_hint("Inner hint")
                .context_menu(crate::Menu::new("", [crate::MenuItem::new("Inner")]))
                .popover(crate::Popover::new(Text::new("Inner popover")))
                .native_element(cx)
        }
    }

    crate::native_view!(Card);

    impl Card {
        fn new() -> Self {
            Self {
                modifiers: crate::ViewModifiers::default(),
            }
        }
    }

    #[test]
    fn composed_view_modifiers_reach_its_root_and_win_over_the_root_s_own() {
        // Verifies modifiers on a composed view replace the ID, width, and declared accessibility of its root.
        let node = Harness::new(
            Card::new()
                .id("outer")
                .width(200.0)
                .accessibility_label("Outer"),
        )
        .tree();
        let RenderNode::View {
            layout,
            id,
            accessibility,
            ..
        } = node.value
        else {
            panic!("expected the card's stack");
        };

        assert_eq!(id.as_deref(), Some("outer"));
        assert_eq!(layout.width, LayoutDimension::Points(200.0));
        assert_eq!(accessibility.label.as_deref(), Some("Outer"));
        // Properties the outer view leaves alone keep the root's.
        assert_eq!(accessibility.hint.as_deref(), Some("Inner hint"));
    }

    #[test]
    fn composed_view_keeps_both_popovers_and_replaces_the_context_menu() {
        // Verifies popovers of the root and the outer view both stay, root first, while the outer menu replaces the root's.
        let children = Harness::new(
            Card::new()
                .context_menu(crate::Menu::new("", [crate::MenuItem::new("Outer")]))
                .popover(crate::Popover::new(Text::new("Outer popover"))),
        )
        .tree()
        .children;
        let menus = children
            .iter()
            .filter_map(|child| match &child.value {
                RenderNode::ContextMenu { items } => Some(items.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let popovers = children
            .iter()
            .filter(|child| matches!(child.value, RenderNode::Popover { .. }))
            .map(|child| child.children[0].clone())
            .collect::<Vec<_>>();

        assert_eq!(menus.len(), 1);
        assert!(format!("{:?}", menus[0]).contains("Outer"));
        assert_eq!(popovers.len(), 2);
        assert!(format!("{:?}", popovers[0]).contains("Inner popover"));
        assert!(format!("{:?}", popovers[1]).contains("Outer popover"));
    }

    #[test]
    fn split_view_takes_the_shared_modifiers() {
        // Verifies a split view names and labels its root stack like any native view.
        let node = Harness::new(
            crate::SplitView::horizontal(Text::new("List"), Text::new("Detail"))
                .id("split")
                .accessibility_label("Panes"),
        )
        .tree();
        let RenderNode::View {
            id, accessibility, ..
        } = node.value
        else {
            panic!("expected the split view's stack");
        };

        assert_eq!(id.as_deref(), Some("split"));
        assert_eq!(accessibility.label.as_deref(), Some("Panes"));
    }

    #[test]
    fn table_with_a_placeholder_keeps_its_id_on_the_table() {
        // Verifies the ID names the native table, not the container that overlays the placeholder.
        let table = Table::new([TableColumn::new("Name")], Vec::<TableRow>::new())
            .empty_content(Text::new("No rows"))
            .id("items");
        let mounted = Mounted::new(table);

        assert_eq!(mounted.view(&[]).1, None);
        assert_eq!(mounted.view(&[0]).1.as_deref(), Some("items"));
    }
}
