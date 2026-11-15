//! Declarative desktop views that render core `RenderNode` descriptions.

/// Implements `View` for native views through their native element.
macro_rules! native_view {
    ($($view:ident),* $(,)?) => {$(
        impl zintl_ui::view::View for $view {
            type Output = $crate::RenderNode;

            fn render(
                &self,
                cx: &mut $crate::Context<'_>,
            ) -> impl zintl_ui::element::IntoElement<Output = Self::Output> {
                $crate::NativeView::native_element(self, cx)
            }
        }
    )*};
}

mod arrangements;
mod attach;
mod button;
mod canvas;
mod chart;
mod choice;
mod container;
mod containers;
mod controls;
mod indicators;
mod list;
mod menu;
mod presentation;
mod sidebar;
mod split_view;
mod stack;
mod table;
pub mod testing;
mod text;
mod text_field;
mod toolbar;
mod types;
mod window;

#[cfg(test)]
mod view_tests;
#[cfg(test)]
mod widget_tests;

pub use arrangements::{DisclosureGroup, Grid, GridCell, ZStack};
pub use attach::{Attach, Control, Layout, LayoutOverrides, NativeView, ViewModifiers};
pub use button::{Button, ButtonStyle};
pub use canvas::Canvas2D;
#[cfg(feature = "wgpu")]
pub use canvas::WgpuCanvas;
pub use chart::{BarChart, ChartBar};
pub use choice::{Picker, PickerStyle, Toggle, ToggleStyle};
pub(crate) use container::Container;
pub use containers::{ScrollView, Tab, TabView};
pub use controls::{
    ColorWell, ComboBox, DatePicker, IntegerField, IntegerValue, NumberField, OptionalDatePicker,
    SearchField, Slider,
};
pub(crate) use controls::{PopUpButton, SegmentedControl, Switch};
pub use indicators::{ProgressIndicator, Separator};
pub use list::{List, ListRow, ListSection};
pub use menu::{
    EnglishMenuLocalizer, JapaneseMenuLocalizer, MainMenu, Menu, MenuItem, MenuItemState,
    MenuLocalizer, MenuRole, MenuState, Modifier, Shortcut, StandardAction, StandardMenu,
    StandardText,
};
pub use presentation::{Alert, AlertButton, ColorPanel, FileDialog, Popover};
pub use sidebar::{Sidebar, SidebarItem, SidebarSection};
pub use split_view::SplitView;
pub use stack::{HStack, Spacer, VStack};
pub use table::{Table, TableColumn, TableRow};
pub use text::Text;
pub use text_field::{TextEditor, TextField};
pub use toolbar::{Toolbar, ToolbarItem};
pub use types::{ButtonRole, ScrollStatus};
pub use window::{UndoHandler, Window};
pub use zintl_ui::element::{Element, IntoElement, ListFactory};
pub use zintl_ui::list;
pub use zintl_ui::store::Store;
pub use zintl_ui_backend_core::{
    Accessibility, AccessibilityRole, AlertStyle, AnnouncementPriority, CalendarDate,
    Canvas2DHandler, CanvasEvent, CanvasEventContext, CanvasId, CanvasNode, CanvasSetup, Color,
    Cursor, DatePickerStyle, DocumentState, Event, EventKind, Frame, GesturePhase, Invalidator,
    Key, KeyEvent, KeyPhase, ListStyle, MagnifyEvent, Modifiers, NamedKey, NodeId, Palette,
    PaletteColor, PointerButton, PointerEvent, PointerPhase, PopoverEdge, Rect, Redraw,
    RedrawPolicy, RenderNode, ScrollAlignment, ScrollAxes, ScrollEvent, ScrollTarget, ShortcutKey,
    Signal, SortOrder, StoreSender, SystemColor, TextAlignment, TextColor, TextStyle, TitleBar,
    UndoState, ViewKind, graphics,
};
// Bezels and button types are AppKit vocabulary, so only the views here name them.
pub(crate) use zintl_ui_backend_core::{BezelStyle, ButtonType};
#[cfg(feature = "wgpu")]
pub use zintl_ui_backend_core::{
    WgpuCanvasHandler, WgpuContext, WgpuError, WgpuSetup, WgpuTarget, wgpu,
};
pub use zintl_ui_layout::{LayoutStyle, Size};

/// An action that receives a control's new value.
pub(crate) type Change<T> = dyn for<'a> Fn(&mut Context<'a>, T);

/// The context views receive while rendering.
pub type Context<'a> = <RenderNode as zintl_ui::renderer::RenderNode>::Context<'a>;

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use zintl_ui::renderer::RenderNode as _;
    use zintl_ui::view::View;
    use zintl_ui_backend_core::ViewKind;
    use zintl_ui_layout::{Axis, LayoutStyle};

    use super::*;
    use crate::testing::Harness;

    fn assert_view<T: View<Output = RenderNode>>() {}

    #[test]
    fn native_widgets_are_zintl_views() {
        // Verifies each native widget participates in declarative composition.
        assert_view::<Button>();
        assert_view::<TextField>();
        assert_view::<Container>();
        assert_view::<Window>();
    }

    #[test]
    fn list_children_preserve_mixed_view_order() {
        // Verifies containers accept ordered heterogeneous list children.
        let children = Harness::new(Container::new(
            LayoutStyle::stack(Axis::Vertical, 8.0),
            list![Button::new("First"), TextField::new(), Button::new("Last")],
        ))
        .tree()
        .children;

        assert_eq!(children.len(), 3);
        assert!(matches!(
            &children[0].value,
            RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { title, .. }), .. } if title == "First"
        ));
        assert!(matches!(
            children[1].value,
            RenderNode::View {
                kind: ViewKind::TextField { .. },
                ..
            }
        ));
        assert!(matches!(
            &children[2].value,
            RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { title, .. }), .. } if title == "Last"
        ));
    }

    #[test]
    fn text_field_presentations_share_one_native_kind() {
        // Verifies labels and editable fields are configurations of one text field node.
        let label = Harness::new(Text::new("Label")).tree().value;
        let field = Harness::new(TextField::new()).tree().value;

        assert!(matches!(
            label,
            RenderNode::View {
                kind: ViewKind::TextField {
                    editable: false,
                    selectable: false,
                    bordered: false,
                    draws_background: false,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            field,
            RenderNode::View {
                kind: ViewKind::TextField {
                    editable: true,
                    selectable: true,
                    bordered: true,
                    draws_background: true,
                    ..
                },
                ..
            }
        ));
        assert!(label.same_kind(&field));
    }

    #[test]
    fn text_field_multiline_is_opt_in() {
        // Verifies text fields are single-line by default and opt into multiline rendering.
        let single_line = Harness::new(TextField::new()).tree().value;
        let multiline = Harness::new(TextField::new().multiline()).tree().value;

        assert!(matches!(
            single_line,
            RenderNode::View {
                kind: ViewKind::TextField {
                    multiline: false,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            multiline,
            RenderNode::View {
                kind: ViewKind::TextField {
                    multiline: true,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn window_title_bar_is_standard_by_default() {
        // Verifies Window::new keeps the standard title bar unless the builder changes it.
        let bounds = Rect::new(0.0, 0.0, 640.0, 480.0);
        let normal = Harness::new(Window::new(bounds, "Normal")).tree().value;
        let full_size = Harness::new(Window::new(bounds, "Hidden").title_bar(TitleBar::Hidden))
            .tree()
            .value;

        assert!(matches!(
            normal,
            RenderNode::Window {
                title_bar: TitleBar::Standard,
                ..
            }
        ));
        assert!(matches!(
            full_size,
            RenderNode::Window {
                title_bar: TitleBar::Hidden,
                ..
            }
        ));
    }

    #[test]
    fn text_binding_and_button_action_route_through_views() {
        // Verifies controls connect native events to Store updates and actions.
        let mut harness = Harness::empty();
        let value = harness.store(String::new());
        let changes = Rc::new(Cell::new(0));
        let received = changes.clone();
        harness.mount(Container::new(
            LayoutStyle::stack(Axis::Vertical, 8.0),
            list![
                TextField::new().bind(value),
                Button::new("Save").on_click(move |_| received.set(received.get() + 1)),
            ],
        ));

        assert!(harness.dispatch(
            &[0, 0],
            Event::TextChanged {
                value: "typed".into(),
            },
        ));
        assert!(harness.dispatch(&[0, 1], Event::Activated));
        assert_eq!(harness.get(value), "typed");
        assert_eq!(changes.get(), 1);
    }

    /// Draws a bar `width` points wide and counts its draws.
    #[derive(Clone)]
    struct Bar {
        width: f64,
        draws: Rc<Cell<u32>>,
    }

    impl Canvas2DHandler for Bar {
        fn draw(&mut self, g: &mut graphics::Graphics2D<'_>, _frame: &Frame) {
            self.draws.set(self.draws.get() + 1);
            g.fill_rect(Rect::new(0.0, 0.0, self.width, 1.0), Color::BLACK);
        }

        fn reconcile(&mut self, next: Self) -> Redraw {
            // Keep the mounted counter so the test sees draws of one handler.
            self.width = next.width;
            Redraw::Yes
        }
    }

    fn canvas(node: &RenderNode) -> CanvasNode {
        match node {
            RenderNode::View {
                kind: ViewKind::Canvas2D { canvas, .. },
                ..
            } => canvas.clone(),
            _ => panic!("expected a canvas node"),
        }
    }

    fn frame() -> Frame {
        Frame {
            bounds: Rect::new(0.0, 0.0, 10.0, 10.0),
            safe_area: Rect::new(0.0, 0.0, 10.0, 10.0),
            scale: 1.0,
            time: 0.0,
            delta: 0.0,
            palette: Default::default(),
        }
    }

    #[test]
    fn canvas_rerender_reconciles_into_the_mounted_handler() {
        // Verifies a Store update keeps the mounted canvas and hands it the new handler.
        let mut harness = Harness::empty();
        let width = harness.store(2.0);
        let draws = Rc::new(Cell::new(0));
        let counter = draws.clone();
        let root = harness.context(|cx| {
            cx.watch(width, move |width| {
                Canvas2D::new(Bar {
                    width: *width,
                    draws: counter.clone(),
                })
            })
        });
        harness.mount(root);
        let mounted = canvas(&harness.tree().value);
        mounted.record(&frame());

        harness.set(width, 5.0);
        let current = canvas(&harness.tree().value);

        assert_eq!(current, mounted);
        let commands = current.record(&frame());
        assert!(
            matches!(&commands[..], [graphics::DrawCommand::Fill { path, .. }]
            if *path == graphics::Path::rect(Rect::new(0.0, 0.0, 5.0, 1.0)))
        );
        assert_eq!(draws.get(), 2);
    }

    #[test]
    fn canvas_events_route_to_on_event() {
        // Verifies canvas input reaches the declared action with a Context.
        let mut harness = Harness::empty();
        let clicks = harness.store(0);
        harness.mount(
            Canvas2D::new(Bar {
                width: 1.0,
                draws: Rc::default(),
            })
            .on_event(move |cx, event| {
                if matches!(
                    event,
                    CanvasEvent::Pointer(PointerEvent {
                        phase: PointerPhase::Down,
                        ..
                    })
                ) {
                    cx.update(clicks, |clicks| *clicks += 1);
                }
            }),
        );
        let down = CanvasEvent::Pointer(PointerEvent {
            phase: PointerPhase::Down,
            button: Some(PointerButton::Primary),
            location: graphics::Point::new(1.0, 1.0),
            modifiers: Modifiers::default(),
            click_count: 1,
        });

        assert!(harness.dispatch(&[0], Event::Canvas(down)));
        assert_eq!(harness.get(clicks), 1);
    }

    /// Emits the click location's x as an `f64` and its count as a `u32`.
    #[derive(Clone)]
    struct Emitter;

    impl Canvas2DHandler for Emitter {
        fn draw(&mut self, _g: &mut graphics::Graphics2D<'_>, _frame: &Frame) {}

        fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
            if let CanvasEvent::Pointer(pointer) = event {
                cx.emit(pointer.location.x);
                cx.emit(pointer.click_count);
            }
            true
        }
    }

    /// Ignores every request a canvas makes.
    struct NullHost;

    impl zintl_ui_backend_core::CanvasHost for NullHost {
        fn invalidate(&self) {}
        fn set_focus(&self, _focused: bool) {}
        fn set_cursor(&self, _cursor: Cursor) {}
        fn actions_emitted(&self, _canvas: CanvasId) {}
    }

    #[test]
    fn canvas_actions_reach_the_on_action_of_their_type() {
        // Verifies each typed on_action sees only its actions, and raw input is not reported without on_event.
        let mut harness = Harness::empty();
        let seen = harness.store(Vec::<String>::new());
        harness.mount(
            Canvas2D::new(Emitter)
                .on_action(move |cx, x: &f64| cx.update(seen, |seen| seen.push(format!("x={x}"))))
                .on_action(move |cx, count: &u32| {
                    cx.update(seen, |seen| seen.push(format!("count={count}")))
                }),
        );
        let node = harness.tree().value;
        let mounted = canvas(&node);
        assert!(matches!(
            node,
            RenderNode::View {
                kind: ViewKind::Canvas2D {
                    reports_input: false,
                    ..
                },
                ..
            }
        ));
        mounted.attach(Rc::new(NullHost), None);
        mounted.event(&CanvasEvent::Pointer(PointerEvent {
            phase: PointerPhase::Down,
            button: Some(PointerButton::Primary),
            location: graphics::Point::new(3.0, 1.0),
            modifiers: Modifiers::default(),
            click_count: 2,
        }));

        assert!(harness.dispatch(
            &[0],
            Event::CanvasActions {
                canvas: mounted.id()
            }
        ));
        assert_eq!(harness.get(seen), ["x=3", "count=2"]);
    }

    #[test]
    fn store_sender_writes_from_another_thread_on_the_main_thread() {
        // Verifies a write sent off the main thread waits for the main-thread task and then updates the Store.
        let mut harness = Harness::empty();
        let tasks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let queue = tasks.clone();
        harness.composer.backend().connect_main_tasks(move |task| {
            queue.lock().unwrap().push(task);
            true
        });
        let status = harness.store("Building".to_owned());
        let sender = harness.context(|cx| cx.sender(status));

        std::thread::spawn(move || assert!(sender.set("Done".to_owned())))
            .join()
            .unwrap();
        assert_eq!(harness.get(status), "Building");
        for task in tasks.lock().unwrap().drain(..) {
            task();
        }
        harness.context(|cx| {
            for update in zintl_ui_backend_core::StoreUpdates::take() {
                update(cx);
            }
        });
        assert_eq!(harness.get(status), "Done");
    }

    #[test]
    fn title_bar_interactive_is_opt_in_per_view() {
        // Verifies only views marked with the modifier keep input under the title bar.
        let plain = Harness::new(Button::new("Plain")).tree().value;
        let marked = Harness::new(Button::new("Marked").title_bar_interactive())
            .tree()
            .value;

        assert!(matches!(
            plain,
            RenderNode::View {
                title_bar_interactive: false,
                ..
            }
        ));
        assert!(matches!(
            marked,
            RenderNode::View {
                title_bar_interactive: true,
                ..
            }
        ));
    }

    #[test]
    fn canvases_of_different_handlers_are_different_kinds() {
        // Verifies switching handler types recreates the canvas instead of reconciling.
        #[derive(Clone)]
        struct Empty;
        impl Canvas2DHandler for Empty {
            fn draw(&mut self, _g: &mut graphics::Graphics2D<'_>, _frame: &Frame) {}
        }
        let bar = Harness::new(Canvas2D::new(Bar {
            width: 1.0,
            draws: Rc::default(),
        }))
        .tree()
        .value;
        let empty = Harness::new(Canvas2D::new(Empty)).tree().value;
        assert!(!bar.same_kind(&empty));
    }

    #[test]
    fn sidebar_binding_filters_unknown_native_ids() {
        // Verifies sidebar selection updates only for declared items.
        let mut harness = Harness::empty();
        let selection = harness.store(None::<String>);
        harness.mount(
            Window::new(Rect::new(0.0, 0.0, 640.0, 480.0), "Navigation").sidebar(
                Sidebar::new([SidebarSection::new([SidebarItem::new("home", "Home")])])
                    .bind(selection),
            ),
        );

        assert!(harness.dispatch(&[0], Event::SidebarSelectionChanged { id: "home".into() },));
        assert_eq!(harness.get(selection), Some("home".into()));
        assert!(harness.dispatch(
            &[0],
            Event::SidebarSelectionChanged {
                id: "missing".into(),
            },
        ));
        assert_eq!(harness.get(selection), Some("home".into()));
    }
}
