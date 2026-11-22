//! Contracts for the controls, presentations, and menus.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use zintl_ui::view::View;
use zintl_ui_backend_core::{
    AlertButton as AlertButtonDescription, AlertButtonRole, CommandItem, CommandRole,
    FileDialogMode, MenuRole as CommandMenuRole, NumberFormat, Sidebar as SidebarDescription,
    WindowStyle,
};
use zintl_ui_layout::{LayoutDimension, LayoutStyle, Size};

use crate::graphics::*;
use crate::testing::{Harness, RenderedNode};
use crate::*;

/// Creates Stores in `init`, publishes them, and renders from them.
struct WithStores<S: Copy + 'static, E> {
    create: fn(&mut Context<'_>) -> S,
    render: Box<dyn Fn(S) -> E>,
    stores: Rc<Cell<Option<S>>>,
}

impl<S: Copy + 'static, E: IntoElement<Output = RenderNode> + 'static> View for WithStores<S, E> {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.stores.set(Some((self.create)(cx)));
    }

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        (self.render)(self.stores.get().expect("init creates the Stores"))
    }
}

impl<S: Copy + 'static, E: IntoElement<Output = RenderNode> + 'static> WithStores<S, E> {
    fn mount(create: fn(&mut Context<'_>) -> S, render: impl Fn(S) -> E + 'static) -> (Harness, S) {
        let stores = Rc::new(Cell::new(None));
        let app = Harness::new(Self {
            create,
            render: Box::new(render),
            stores: stores.clone(),
        });
        let stores = stores.get().expect("mounting runs init");
        (app, stores)
    }
}

/// Records how many times an action ran.
#[derive(Clone, Default)]
struct Calls(Rc<Cell<usize>>);

impl Calls {
    fn action(&self) -> impl for<'a> Fn(&mut Context<'a>) + 'static {
        let calls = self.0.clone();
        move |_| calls.set(calls.get() + 1)
    }

    fn count(&self) -> usize {
        self.0.get()
    }
}

#[test]
fn bar_lengths_follow_their_values() {
    // Verifies bars are proportional to their values, the largest fills the track, and negatives show none.
    let bars = [
        ChartBar::new("Food", 3480.0),
        ChartBar::new("Transport", 1740.0),
        ChartBar::new("Refund", -20.0),
    ];
    assert_eq!(BarChart::new(bars).fractions(), [1.0, 0.5, 0.0]);
    assert_eq!(
        BarChart::new([ChartBar::new("Empty", 0.0)]).fractions(),
        [0.0]
    );
}

#[test]
fn bar_chart_maximum_sets_the_value_that_fills_the_track() {
    // Verifies bars are a share of the given maximum, and values above it fill the track.
    let chart = BarChart::new([
        ChartBar::new("Utilities", 52.0),
        ChartBar::new("Food", 25.0),
        ChartBar::new("Over", 150.0),
    ])
    .maximum(100.0);
    assert_eq!(chart.fractions(), [0.52, 0.25, 1.0]);
}

#[test]
fn bar_chart_estimates_wide_characters_as_one_em() {
    // Verifies CJK labels are estimated at the font size per character, so short labels are not truncated.
    assert!(BarChart::text_width("給与") >= 2.0 * 13.0);
    assert!(BarChart::text_width("水道光熱費") >= 5.0 * 13.0);
    assert!(BarChart::text_width("Food") < BarChart::text_width("給与給与"));
}

/// Returns the chart's group node and its rows' bar canvases.
fn chart_parts(chart: BarChart) -> (RenderNode, Vec<RenderNode>) {
    let tree = Harness::new(chart).tree();
    let bars = tree
        .children
        .iter()
        // Each row is a label column, the track, and a value column; the track holds the bar.
        .map(|row| row.children[1].children[0].children[0].value.clone())
        .collect();
    (tree.value, bars)
}

#[test]
fn bar_chart_exposes_a_group_with_bar_ids_and_fixed_columns() {
    // Verifies the chart is a named group, bars carry their IDs, and fixed column widths are kept.
    let chart = BarChart::new([
        ChartBar::new("Food", 3480.0).id("expenses-food"),
        ChartBar::new("Other", 300.0).id("expenses-other"),
    ])
    .label_width(160.0)
    .value_width(96.0)
    .id("expenses");
    let tree = Harness::new(chart.clone()).tree();
    assert!(matches!(
        &tree.value,
        RenderNode::View { id: Some(id), accessibility, .. }
            if id == "expenses" && accessibility.role == Some(AccessibilityRole::Group)
    ));
    let widths = |row: &RenderedNode| {
        [0, 2].map(|column| match &row.children[column].value {
            RenderNode::View { layout, .. } => layout.width,
            _ => LayoutDimension::Auto,
        })
    };
    assert_eq!(
        widths(&tree.children[0]),
        [
            LayoutDimension::Points(160.0),
            LayoutDimension::Points(96.0)
        ]
    );
    let (_, bars) = chart_parts(chart);
    let ids = bars
        .iter()
        .map(|bar| match bar {
            RenderNode::View { id, .. } => id.clone(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [Some("expenses-food".into()), Some("expenses-other".into())]
    );
}

#[test]
fn bar_chart_defaults_to_the_accent_color() {
    // Verifies bars follow the accent color unless they choose a system or fixed color.
    let palette = Palette::LIGHT;
    let frame = Frame {
        palette: Palette {
            accent: Color::rgb(1.0, 0.0, 1.0),
            ..palette
        },
        ..canvas_frame()
    };
    let (_, bars) = chart_parts(BarChart::new([
        ChartBar::new("Food", 1.0),
        ChartBar::new("Refunds", 1.0).color(SystemColor::Green),
    ]));
    let fills = bars
        .iter()
        .map(|bar| {
            let RenderNode::View {
                kind: ViewKind::Canvas2D { canvas, .. },
                ..
            } = bar
            else {
                panic!("bars are canvases");
            };
            let commands = canvas.record(&frame);
            commands
                .iter()
                .find_map(|command| match command {
                    graphics::DrawCommand::Fill { paint, .. } => Some(paint.clone()),
                    _ => None,
                })
                .expect("the bar fills its frame")
        })
        .collect::<Vec<_>>();
    assert_eq!(fills[0], Color::rgb(1.0, 0.0, 1.0).into());
    assert_eq!(fills[1], palette.green.into());
}

#[test]
fn system_text_colors_reach_the_render_node() {
    // Verifies a system color is kept by name, so the backend resolves it for the current appearance.
    let tree = Harness::new(Text::new("Error").color(TextColor::System(SystemColor::Red))).tree();
    assert!(matches!(
        tree.value,
        RenderNode::View {
            kind: ViewKind::TextField { style, .. },
            ..
        } if style.color == TextColor::System(SystemColor::Red)
    ));
}

#[test]
fn icon_only_buttons_leave_their_size_to_the_backend() {
    // Verifies a symbol-only button drops the 80 x 32 push button minimum, unless one is declared.
    let minimum = |button: Button| match Harness::new(button).tree().value {
        RenderNode::View { layout, .. } => layout.minimum_size,
        _ => None,
    };
    assert_eq!(
        minimum(Button::icon("chevron.left", "Previous")),
        Some(Size::default())
    );
    assert_eq!(
        minimum(Button::icon("chevron.left", "Previous").minimum_size(Size::new(44.0, 30.0))),
        Some(Size::new(44.0, 30.0))
    );
    assert_eq!(minimum(Button::new("Save")), Some(Size::new(80.0, 32.0)));
}

#[test]
fn switch_toggle_round_trips_through_its_store() {
    // Verifies an unlabeled Toggle renders a switch that writes user changes to its Store.
    let (mut app, on) = WithStores::mount(|cx| cx.store(true), |on| Toggle::new("").bind(on));
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::Switch { on: true, .. },
            ..
        }
    ));

    assert!(app.dispatch(&[0], Event::StateChanged { on: false }));

    assert!(!app.get(on));
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::Switch { on: false, .. },
            ..
        }
    ));
}

#[test]
fn labeled_toggle_puts_its_id_on_the_switch_and_sizes_on_the_row() {
    // Verifies the switch carries the ID, the labeled row takes the size, and both are disabled.
    let row = Harness::new(Toggle::new("Wi-Fi").id("wifi").enabled(false).width(200.0)).tree();

    let RenderNode::View {
        id,
        enabled,
        layout,
        ..
    } = &row.value
    else {
        panic!("expected a row, got {:?}", row.value);
    };
    assert_eq!(id, &None);
    assert!(!enabled);
    assert_eq!(layout.width, LayoutDimension::Points(200.0));
    assert!(matches!(
        &row.children[1].value,
        RenderNode::View { kind: ViewKind::Switch { .. }, id: Some(id), enabled: false, .. } if id == "wifi"
    ));
}

#[test]
fn toggle_styles_use_native_two_state_controls() {
    // Verifies switch rows lead with the label and other styles use button types.
    let row = Harness::new(Toggle::new("Wi-Fi")).tree();
    assert!(matches!(
        row.value,
        RenderNode::View {
            kind: ViewKind::Container,
            ..
        }
    ));
    assert!(matches!(
        &row.children[0].value,
        RenderNode::View { kind: ViewKind::TextField { value, .. }, .. } if value == "Wi-Fi"
    ));
    assert!(matches!(
        row.children[1].value,
        RenderNode::View {
            kind: ViewKind::Switch { .. },
            ..
        }
    ));

    let checkbox = Harness::new(Toggle::new("Agree").style(ToggleStyle::Checkbox))
        .tree()
        .value;
    let button = Harness::new(Toggle::new("Bold").style(ToggleStyle::Button))
        .tree()
        .value;
    assert!(matches!(
        checkbox,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                button_type: ButtonType::Checkbox,
                ..
            }),
            ..
        }
    ));
    assert!(matches!(
        button,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                button_type: ButtonType::PushOnPushOff,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn radio_picker_selects_exactly_one_option() {
    // Verifies radio buttons reflect the Store and a newly checked radio selects its index.
    let (mut app, selected) = WithStores::mount(
        |cx| cx.store(0_usize),
        |selected| {
            Picker::indexed(["Low", "Normal", "High"])
                .style(PickerStyle::Radio)
                .bind(selected)
        },
    );
    let states = |app: &Harness| {
        app.tree()
            .children
            .iter()
            .map(|radio| match radio.value {
                RenderNode::View {
                    kind:
                        ViewKind::Button(zintl_ui_backend_core::Button {
                            button_type: ButtonType::Radio,
                            on,
                            ..
                        }),
                    ..
                } => on,
                _ => panic!("radio pickers render radio buttons"),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(states(&app), [true, false, false]);

    assert!(app.dispatch(&[0, 2], Event::StateChanged { on: true }));

    assert_eq!(app.get(selected), 2);
    assert_eq!(states(&app), [false, false, true]);
}

#[test]
fn menu_and_segmented_pickers_render_bound_selection() {
    // Verifies other picker styles map to pop-up buttons and segmented controls.
    let (mut app, selected) = WithStores::mount(
        |cx| cx.store(1_usize),
        |selected| Picker::indexed(["A", "B"]).bind(selected),
    );
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::PopUpButton {
                selected: Some(1),
                ..
            },
            ..
        }
    ));
    assert!(app.dispatch(&[0], Event::SelectionChanged { index: 0 }));
    assert_eq!(app.get(selected), 0);

    let segmented = Harness::new(Picker::indexed(["A", "B"]).style(PickerStyle::Segmented))
        .tree()
        .value;
    assert!(matches!(
        segmented,
        RenderNode::View { kind: ViewKind::SegmentedControl { segments, .. }, .. } if segments == ["A", "B"]
    ));
}

/// A value type for pickers and tab views that select by value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Speed {
    Slow,
    Normal,
    Fast,
}

impl Speed {
    const ALL: [(Speed, &'static str); 3] = [
        (Speed::Slow, "Slow"),
        (Speed::Normal, "Normal"),
        (Speed::Fast, "Fast"),
    ];
}

/// Returns the option index a picker of any style shows as selected.
fn picker_selection(node: &RenderedNode) -> Option<usize> {
    match &node.value {
        RenderNode::View {
            kind:
                ViewKind::PopUpButton { selected, .. } | ViewKind::SegmentedControl { selected, .. },
            ..
        } => *selected,
        // A radio group: the index of the radio that is on, if exactly one is.
        _ => {
            let on: Vec<usize> = node
                .children
                .iter()
                .enumerate()
                .filter(|(_, radio)| {
                    matches!(
                        radio.value,
                        RenderNode::View {
                            kind: ViewKind::Button(zintl_ui_backend_core::Button { on: true, .. }),
                            ..
                        }
                    )
                })
                .map(|(index, _)| index)
                .collect();
            assert!(on.len() <= 1, "a radio picker turns on one radio at most");
            on.first().copied()
        }
    }
}

#[test]
fn picker_shows_the_option_with_the_selected_value_in_every_style() {
    // Verifies a value becomes the index of its option, and a missing value selects nothing.
    for style in [
        PickerStyle::Menu,
        PickerStyle::Segmented,
        PickerStyle::Radio,
    ] {
        let shown =
            |picker: Picker<Speed>| picker_selection(&Harness::new(picker.style(style)).tree());

        assert_eq!(
            shown(Picker::new(Speed::ALL).selection(Speed::Fast)),
            Some(2),
            "{style:?}"
        );
        assert_eq!(
            shown(Picker::new(Speed::ALL)),
            Some(0),
            "{style:?} without a value"
        );
        assert_eq!(
            shown(Picker::new(Speed::ALL[..2].to_vec()).selection(Speed::Fast)),
            None,
            "{style:?} with a value no option has"
        );
    }
}

#[test]
fn picker_follows_its_value_when_options_are_reordered() {
    // Verifies the bound value, not its old position, picks the option after the options move.
    let (mut app, (speed, reversed)) = WithStores::mount(
        |cx| (cx.store(Speed::Slow), cx.store(false)),
        |(speed, reversed)| {
            WithReversed(reversed, move |reversed: bool| {
                let mut options = Speed::ALL.to_vec();
                if reversed {
                    options.reverse();
                }
                Picker::new(options).bind(speed)
            })
        },
    );
    assert_eq!(picker_selection(&app.tree()), Some(0));

    app.set(reversed, true);

    assert_eq!(picker_selection(&app.tree()), Some(2));
    assert_eq!(app.get(speed), Speed::Slow);
}

/// Renders a view built from the current value of a `bool` Store.
#[derive(Clone)]
struct WithReversed<F>(Store<bool>, F);

impl<F, E> View for WithReversed<F>
where
    F: Fn(bool) -> E + Clone + 'static,
    E: IntoElement<Output = RenderNode> + 'static,
{
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        (self.1)(*cx.get(self.0))
    }
}

#[test]
fn picker_reports_the_value_of_the_chosen_option() {
    // Verifies a selected index writes that option's value to the Store and passes it to `on_change`.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = seen.clone();
    let (mut app, speed) = WithStores::mount(
        |cx| cx.store(Speed::Slow),
        move |speed| {
            let record = record.clone();
            Picker::new(Speed::ALL)
                .bind(speed)
                .on_change(move |_, value| record.borrow_mut().push(*value))
        },
    );

    assert!(app.dispatch(&[0], Event::SelectionChanged { index: 1 }));

    assert_eq!(app.get(speed), Speed::Normal);
    assert_eq!(*seen.borrow(), [Speed::Normal]);
    assert_eq!(picker_selection(&app.tree()), Some(1));
}

#[test]
fn picker_ignores_indexes_past_its_options() {
    // Verifies a stale index from the backend neither writes the Store nor calls `on_change`.
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    let (mut app, speed) = WithStores::mount(
        |cx| cx.store(Speed::Slow),
        move |speed| {
            let count = count.clone();
            Picker::new(Speed::ALL)
                .bind(speed)
                .on_change(move |_, _| count.set(count.get() + 1))
        },
    );

    app.dispatch(&[0], Event::SelectionChanged { index: 3 });

    assert_eq!(app.get(speed), Speed::Slow);
    assert_eq!(changes.get(), 0);
}

#[test]
fn radio_picker_reports_the_value_of_the_radio_turned_on() {
    // Verifies a radio turning on selects its option's value.
    let (mut app, speed) = WithStores::mount(
        |cx| cx.store(Speed::Slow),
        |speed| {
            Picker::new(Speed::ALL)
                .style(PickerStyle::Radio)
                .bind(speed)
        },
    );

    assert!(app.dispatch(&[0, 2], Event::StateChanged { on: true }));

    assert_eq!(app.get(speed), Speed::Fast);
    assert_eq!(picker_selection(&app.tree()), Some(2));
}

/// Returns the page index a tab view shows.
fn tab_selection(node: &RenderedNode) -> usize {
    match node.value {
        RenderNode::View {
            kind: ViewKind::TabView { selected, .. },
            ..
        } => selected,
        _ => panic!("expected a tab view"),
    }
}

/// Builds a tab view with one page per speed, in `order`.
fn speed_tabs(order: impl IntoIterator<Item = (Speed, &'static str)>) -> TabView<Speed> {
    TabView::new(
        order
            .into_iter()
            .map(|(speed, title)| Tab::new(speed, title, Text::new(title))),
    )
}

#[test]
fn tab_view_shows_the_page_with_the_selected_value() {
    // Verifies a value picks its page, and no value or an unknown one shows the first page.
    let shown = |tabs: TabView<Speed>| tab_selection(&Harness::new(tabs).tree());

    assert_eq!(shown(speed_tabs(Speed::ALL).selection(Speed::Fast)), 2);
    assert_eq!(shown(speed_tabs(Speed::ALL)), 0);
    assert_eq!(
        shown(speed_tabs(Speed::ALL[..2].to_vec()).selection(Speed::Fast)),
        0
    );
}

#[test]
fn tab_view_shows_the_first_page_without_changing_an_unknown_bound_value() {
    // Verifies the fallback to the first page leaves the Store as the app wrote it.
    let (mut app, speed) = WithStores::mount(
        |cx| cx.store(Speed::Fast),
        |speed| speed_tabs(Speed::ALL[..2].to_vec()).bind(speed),
    );

    assert_eq!(tab_selection(&app.tree()), 0);
    assert_eq!(app.get(speed), Speed::Fast);
}

#[test]
fn tab_view_follows_its_value_when_tabs_are_reordered() {
    // Verifies the bound value, not its old position, picks the page after the tabs move.
    let (mut app, (_, reversed)) = WithStores::mount(
        |cx| (cx.store(Speed::Slow), cx.store(false)),
        |(speed, reversed)| {
            WithReversed(reversed, move |reversed: bool| {
                let mut order = Speed::ALL.to_vec();
                if reversed {
                    order.reverse();
                }
                speed_tabs(order).bind(speed)
            })
        },
    );
    assert_eq!(tab_selection(&app.tree()), 0);

    app.set(reversed, true);

    assert_eq!(tab_selection(&app.tree()), 2);
}

#[test]
fn tab_view_reports_the_value_of_the_chosen_page() {
    // Verifies a selected index writes that page's value and passes it to `on_change`, ignoring stale indexes.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = seen.clone();
    let (mut app, speed) = WithStores::mount(
        |cx| cx.store(Speed::Slow),
        move |speed| {
            let record = record.clone();
            speed_tabs(Speed::ALL)
                .bind(speed)
                .on_change(move |_, value| record.borrow_mut().push(*value))
        },
    );

    assert!(app.dispatch(&[0], Event::SelectionChanged { index: 1 }));
    app.dispatch(&[0], Event::SelectionChanged { index: 3 });

    assert_eq!(app.get(speed), Speed::Normal);
    assert_eq!(*seen.borrow(), [Speed::Normal]);
    assert_eq!(tab_selection(&app.tree()), 1);
}

/// Formats kilometres as `12k345.67`.
fn format_km(value: f64) -> String {
    let km = (value / 1000.0).floor();
    format!("{km}k{:06.2}", value - km * 1000.0)
}

/// Reads `12k345.67` and plain `12345.67`.
fn parse_km(text: &str) -> Option<f64> {
    match text.split_once('k') {
        Some((km, m)) => Some(km.parse::<f64>().ok()? * 1000.0 + m.parse::<f64>().ok()?),
        None => text.parse().ok(),
    }
}

#[test]
fn number_field_shows_text_from_its_formatter_and_suffix() {
    // Verifies a custom formatter replaces platform formatting and the suffix follows the text.
    let (app, _) = WithStores::mount(
        |cx| cx.store(12345.67_f64),
        |value| {
            NumberField::new()
                .formatter(format_km)
                .suffix(" m")
                .bind(value)
        },
    );
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::NumberField { text: Some(text), .. },
            ..
        } if text == "12k345.67 m"
    ));
}

#[test]
fn number_field_parses_committed_text_and_ignores_invalid_text() {
    // Verifies committed text goes through the parser, with or without the suffix, and bad text keeps the value.
    let (mut app, value) = WithStores::mount(
        |cx| cx.store(0.0_f64),
        |value| {
            NumberField::new()
                .formatter(format_km)
                .parser(parse_km)
                .suffix(" m")
                .range(0.0..=50_000.0)
                .bind(value)
        },
    );
    let commit = |text: &str| Event::TextCommitted { value: text.into() };

    app.dispatch(&[0], commit("1k250.5 m"));
    assert_eq!(app.get(value), 1250.5);
    app.dispatch(&[0], commit("2000"));
    assert_eq!(app.get(value), 2000.0);
    app.dispatch(&[0], commit("not a number"));
    assert_eq!(app.get(value), 2000.0);
    app.dispatch(&[0], commit("90k0"));
    assert_eq!(app.get(value), 50_000.0);
}

#[test]
fn text_field_commit_runs_only_when_editing_ends() {
    // Verifies on_commit ignores keystrokes and runs with the final text when editing ends.
    let (mut app, committed) = WithStores::mount(
        |cx| cx.store(Vec::<String>::new()),
        |committed| {
            TextField::new().on_commit(move |cx, text| {
                let text = text.to_owned();
                cx.update(committed, |committed| committed.push(text));
            })
        },
    );

    app.dispatch(
        &[0],
        Event::TextChanged {
            value: "Sec".into(),
        },
    );
    assert!(app.get(committed).is_empty());
    app.dispatch(
        &[0],
        Event::TextCommitted {
            value: "Section A".into(),
        },
    );
    assert_eq!(app.get(committed), ["Section A"]);
}

#[test]
fn integer_field_parses_and_formats_exact_integers() {
    // Verifies grouping, signs, full-width digits, and range limits, and that decimals are rejected.
    let field = IntegerField::new().grouping(true);
    assert_eq!(field.format(1234567), "1,234,567");
    assert_eq!(field.format(-1234), "-1,234");
    assert_eq!(field.format(999), "999");
    assert_eq!(field.parse("1,234"), Some(1234));
    assert_eq!(field.parse("１２３４"), Some(1234));
    assert_eq!(field.parse("－５"), Some(-5));
    assert_eq!(field.parse("12.5"), None);
    assert_eq!(field.parse("abc"), None);
    // 2^53 + 1 stays exact because it never becomes an f64.
    assert_eq!(field.parse("9007199254740993"), Some(9_007_199_254_740_993));
    let limited = IntegerField::new().range(0..=100).suffix(" pcs");
    assert_eq!(limited.parse("250 pcs"), Some(100));
    assert_eq!(limited.parse("-3"), Some(0));
    assert_eq!(limited.format(7), "7 pcs");
}

#[test]
fn integer_field_steps_by_its_increment() {
    // Verifies a stepper click moves the bound value by the step, whatever value the stepper reports.
    let (mut app, value) = WithStores::mount(
        |cx| cx.store(9_007_199_254_740_993_i64),
        |value| IntegerField::new().step(2).bind(value),
    );
    assert!(app.dispatch(&[0], Event::ValueChanged { value: 9.1e15 }));
    assert_eq!(app.get(value), 9_007_199_254_740_995);
    assert!(app.dispatch(&[0], Event::ValueChanged { value: 0.0 }));
    assert_eq!(app.get(value), 9_007_199_254_740_993);
}

#[test]
fn integer_field_reports_typed_integers_before_editing_ends() {
    // Verifies typed text that is an integer reaches the Store at once, and partial text does not.
    let (mut app, value) = WithStores::mount(
        |cx| cx.store(3_i64),
        |value| IntegerField::new().grouping(true).bind(value),
    );
    app.dispatch(&[0], Event::TextChanged { value: "42".into() });
    assert_eq!(app.get(value), 42);
    for partial in ["-", "", "4x"] {
        app.dispatch(
            &[0],
            Event::TextChanged {
                value: partial.into(),
            },
        );
        assert_eq!(app.get(value), 42, "{partial:?}");
    }
    app.dispatch(&[0], Event::TextChanged { value: "1,".into() });
    assert_eq!(app.get(value), 1);
    app.dispatch(
        &[0],
        Event::TextChanged {
            value: "1,234".into(),
        },
    );
    assert_eq!(app.get(value), 1234);
    // The rendered text is formatted; the backend keeps typed text until editing ends.
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::NumberField { text: Some(text), .. },
            ..
        } if text == "1,234"
    ));
}

#[test]
fn optional_integer_field_treats_empty_text_as_none() {
    // Verifies empty text parses to None, zero stays a value, and the field shows None as empty text.
    let field = IntegerField::optional();
    assert_eq!(field.parse(""), Some(None));
    assert_eq!(field.parse("  "), Some(None));
    assert_eq!(field.parse("0"), Some(Some(0)));
    assert_eq!(field.parse("abc"), None);
    assert_eq!(field.format(None), "");
    // Fields bound to `i64` reject empty text instead.
    assert_eq!(IntegerField::new().parse(""), None);

    let (mut app, value) = WithStores::mount(
        |cx| cx.store(Some(7_i64)),
        |value| IntegerField::optional().placeholder("None").bind(value),
    );
    app.dispatch(&[0], Event::TextChanged { value: "".into() });
    assert_eq!(app.get(value), None);
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::NumberField { text: Some(text), placeholder: Some(placeholder), value, .. },
            ..
        } if text.is_empty() && placeholder == "None" && value == 0.0
    ));
}

#[test]
fn optional_integer_field_steps_from_zero_when_empty() {
    // Verifies the first stepper click on an empty field starts from zero, limited to the range.
    let (mut app, value) = WithStores::mount(
        |cx| cx.store(None::<i64>),
        |value| IntegerField::optional().bind(value),
    );
    assert!(app.dispatch(&[0], Event::ValueChanged { value: 1.0 }));
    assert_eq!(app.get(value), Some(1));

    let (mut app, value) = WithStores::mount(
        |cx| cx.store(None::<i64>),
        |value| IntegerField::optional().range(5..=9).bind(value),
    );
    assert!(app.dispatch(&[0], Event::ValueChanged { value: 4.0 }));
    assert_eq!(app.get(value), Some(5));
}

#[test]
fn optional_date_picker_clears_and_picks_dates() {
    // Verifies the clear button routes None, the placeholder button picks today, and picks route Some.
    let leap = CalendarDate::new(2024, 2, 29).unwrap();
    let (mut app, date) = WithStores::mount(
        move |cx| cx.store(Some(CalendarDate::new(2026, 11, 15).unwrap())),
        |date| {
            DatePicker::optional()
                .placeholder("No date")
                .bind(date)
                .id("due")
        },
    );
    let tree = app.tree();
    assert!(matches!(
        &tree.children[..],
        [picker, clear] if matches!(
            &picker.value,
            RenderNode::View { kind: ViewKind::DatePicker { .. }, id: Some(id), .. } if id == "due"
        ) && matches!(
            &clear.value,
            RenderNode::View { id: Some(id), .. } if id == "due-clear"
        )
    ));
    assert!(app.dispatch(&[0, 0], Event::DateChanged { date: leap }));
    assert_eq!(app.get(date), Some(leap));

    assert!(app.dispatch(&[0, 1], Event::Activated));
    assert_eq!(app.get(date), None);
    let tree = app.tree();
    assert!(matches!(
        &tree.children[..],
        [button] if matches!(
            &button.value,
            RenderNode::View { kind: ViewKind::Button(button), id: Some(id), .. }
                if button.title == "No date" && id == "due"
        )
    ));

    assert!(app.dispatch(&[0, 0], Event::Activated));
    assert_eq!(app.get(date), Some(CalendarDate::today()));
}

#[test]
fn number_field_clamps_entered_values() {
    // Verifies values outside the declared range are clamped before reaching the Store.
    let (mut app, value) = WithStores::mount(
        |cx| cx.store(5.0_f64),
        |value| NumberField::new().range(0.0..=10.0).bind(value),
    );

    assert!(app.dispatch(&[0], Event::ValueChanged { value: 42.0 }));

    assert_eq!(app.get(value), 10.0);
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::NumberField {
                value: 10.0,
                format: NumberFormat {
                    minimum: 0.0,
                    maximum: 10.0,
                    ..
                },
                text: None,
                placeholder: None,
            },
            ..
        }
    ));
}

#[test]
fn slider_combo_box_search_and_color_write_their_stores() {
    // Verifies each value control forwards its native event payload to its Store.
    #[derive(Clone, Copy)]
    struct Stores {
        volume: Store<f64>,
        fruit: Store<String>,
        query: Store<String>,
        tint: Store<Color>,
    }
    let (mut app, stores) = WithStores::mount(
        |cx| Stores {
            volume: cx.store(0.0),
            fruit: cx.store(String::new()),
            query: cx.store(String::new()),
            tint: cx.store(Color::BLACK),
        },
        |stores| {
            VStack::new(list![
                Slider::new(0.0..=100.0).bind(stores.volume),
                ComboBox::new(["Apple"]).bind(stores.fruit),
                SearchField::new().bind(stores.query),
                ColorWell::new().bind(stores.tint),
            ])
        },
    );

    app.dispatch(&[0, 0], Event::ValueChanged { value: 30.0 });
    app.dispatch(
        &[0, 1],
        Event::TextChanged {
            value: "Apple".into(),
        },
    );
    app.dispatch(&[0, 2], Event::TextChanged { value: "ap".into() });
    app.dispatch(
        &[0, 3],
        Event::ColorChanged {
            color: Color::WHITE,
        },
    );

    assert_eq!(app.get(stores.volume), 30.0);
    assert_eq!(app.get(stores.fruit), "Apple");
    assert_eq!(app.get(stores.query), "ap");
    assert_eq!(app.get(stores.tint), Color::WHITE);
}

#[test]
fn color_well_shows_its_declared_color_and_reports_changes() {
    // Verifies an unbound color well shows `value` and passes picked colors to `on_change`.
    let red = Color::rgba(1.0, 0.0, 0.0, 1.0);
    let picked = Rc::new(Cell::new(None));
    let mut app = Harness::new(VStack::new(list![ColorWell::new().value(red).on_change({
        let picked = picked.clone();
        move |_, color| picked.set(Some(color))
    })]));
    assert!(matches!(
        app.tree().children[0].value,
        RenderNode::View {
            kind: ViewKind::ColorWell { color },
            ..
        } if color == red
    ));

    assert!(app.dispatch(
        &[0, 0],
        Event::ColorChanged {
            color: Color::WHITE,
        },
    ));
    assert_eq!(picked.get(), Some(Color::WHITE));
}

#[test]
fn button_title_color_reaches_the_render_node() {
    // Verifies a title color is declared on the button, and plain buttons leave it to the platform.
    let red = Color::rgba(0.9, 0.1, 0.1, 1.0);
    let tree = Harness::new(VStack::new(list![
        Button::new("Problem").title_color(red),
        Button::new("Plain"),
    ]))
    .tree();
    let colors: Vec<_> = tree
        .children
        .iter()
        .map(|child| match &child.value {
            RenderNode::View {
                kind: ViewKind::Button(button),
                ..
            } => button.title_color,
            other => panic!("expected a button, got {other:?}"),
        })
        .collect();
    assert_eq!(colors, vec![Some(red), None]);
}

#[test]
fn button_roles_and_styles_configure_nsbutton() {
    // Verifies roles become key equivalents or destructive styling and styles pick a bezel.
    let render = |button: Button| Harness::new(button).tree().value;
    assert!(matches!(
        render(Button::new("OK").role(ButtonRole::Default)),
        RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { key_equivalent: Some(key), .. }), .. } if key == "\r"
    ));
    assert!(matches!(
        render(Button::new("Cancel").role(ButtonRole::Cancel)),
        RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { key_equivalent: Some(key), .. }), .. } if key == "\u{1b}"
    ));
    assert!(matches!(
        render(Button::new("Delete").role(ButtonRole::Destructive)),
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                destructive: true,
                ..
            }),
            ..
        }
    ));
    assert!(matches!(
        render(
            Button::icon("gearshape", "Settings")
                .style(ButtonStyle::Borderless)
        ),
        RenderNode::View { kind: ViewKind::Button(zintl_ui_backend_core::Button { bezel: None, system_image: Some(name), .. }), .. } if name == "gearshape"
    ));
}

#[test]
fn context_menu_routes_custom_items_by_position() {
    // Verifies context menus attach to their view, assign IDs, and keep actionless items disabled.
    let calls = Calls::default();
    let mut app = Harness::new(Button::new("Item").context_menu([
        MenuItem::new("Rename").on_select(calls.action()),
        MenuItem::separator(),
        MenuItem::standard(StandardAction::Copy),
        MenuItem::new("Unavailable"),
    ]));
    let tree = app.tree();
    let RenderNode::ContextMenu { items } = &tree.children[0].value else {
        panic!("context menus render as a child context menu");
    };
    assert!(matches!(
        &items[0],
        CommandItem { id: Some(id), enabled: true, .. } if id == "context.0"
    ));
    assert_eq!(items[1].role, Some(CommandRole::Separator));
    assert!(matches!(
        &items[2],
        CommandItem {
            id: None,
            role: Some(CommandRole::Copy),
            key: Some(_),
            ..
        }
    ));
    assert!(matches!(&items[3], CommandItem { enabled: false, .. }));

    assert!(app.dispatch(
        &[0, 0],
        Event::MenuItemSelected {
            id: "context.0".into(),
        }
    ));
    assert_eq!(calls.count(), 1);
}

#[test]
fn view_builders_chain_after_attachments() {
    // Verifies attachment modifiers keep the view type, so its own builders still apply.
    let app = Harness::new(
        VStack::new(list![Text::new("Row")])
            .context_menu([MenuItem::new("Duplicate")])
            .spacing(12.0)
            .padding(20.0),
    );
    let tree = app.tree();
    let RenderNode::View {
        kind: ViewKind::Container,
        layout,
        ..
    } = &tree.value
    else {
        panic!("stacks render as a container view");
    };
    assert_eq!((layout.gap, layout.padding), (12.0, 20.0));
    assert!(matches!(
        tree.children.last().map(|child| &child.value),
        Some(RenderNode::ContextMenu { .. })
    ));
}

#[test]
fn popover_follows_its_presented_store() {
    // Verifies popovers render content as children and native dismissal clears the Store.
    let (mut app, shown) = WithStores::mount(
        |cx| cx.store(false),
        |shown| {
            Button::new("Info").popover(Popover::new(Text::new("Details")).bind_presented(shown))
        },
    );
    let popover = |app: &Harness| app.tree().children[0].clone();
    assert!(matches!(
        popover(&app).value,
        RenderNode::Popover {
            presented: false,
            ..
        }
    ));
    assert_eq!(popover(&app).children.len(), 1);

    app.set(shown, true);
    assert!(matches!(
        popover(&app).value,
        RenderNode::Popover {
            presented: true,
            ..
        }
    ));

    assert!(app.dispatch(&[0, 0], Event::PopoverClosed));
    assert!(!app.get(shown));
}

#[test]
fn alert_buttons_run_their_action_and_dismiss() {
    // Verifies a chosen alert button runs once, while a programmatic dismissal runs nothing.
    let deletes = Calls::default();
    let action = deletes.clone();
    let (mut app, shown) = WithStores::mount(
        |cx| cx.store(true),
        move |shown| {
            Window::new("Alerts").size(Size::new(400.0, 300.0)).alert(
                Alert::new("Delete?")
                    .button(
                        AlertButton::new("Delete")
                            .role(ButtonRole::Destructive)
                            .on_click(action.action()),
                    )
                    .button(AlertButton::new("Cancel").role(ButtonRole::Cancel))
                    .bind_presented(shown),
            )
        },
    );
    let tree = app.tree();
    assert!(matches!(
        &tree.children[0].value,
        RenderNode::Alert { presented: true, config }
            if config.buttons == [
                AlertButtonDescription {
                    title: "Delete".into(),
                    role: AlertButtonRole::Destructive,
                },
                AlertButtonDescription {
                    title: "Cancel".into(),
                    role: AlertButtonRole::Cancel,
                },
            ]
    ));

    assert!(app.dispatch(&[0, 0], Event::AlertResponse { button: Some(0) }));
    assert_eq!(deletes.count(), 1);
    assert!(!app.get(shown));

    app.set(shown, true);
    app.dispatch(&[0, 0], Event::AlertResponse { button: None });
    assert_eq!(deletes.count(), 1);
    assert!(!app.get(shown));
}

#[test]
fn file_dialog_reports_paths_or_cancellation() {
    // Verifies save panels report the chosen path, cancellation, and clear their Store.
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let cancels = Calls::default();
    let (received, cancel) = (chosen.clone(), cancels.clone());
    let (mut app, shown) = WithStores::mount(
        |cx| cx.store(true),
        move |shown| {
            let received = received.clone();
            Window::new("Files")
                .size(Size::new(400.0, 300.0))
                .file_dialog(
                    FileDialog::save()
                        .file_name("notes.txt")
                        .bind_presented(shown)
                        .on_select(move |_, paths| received.borrow_mut().extend_from_slice(paths))
                        .on_cancel(cancel.action()),
                )
        },
    );
    assert!(matches!(
        &app.tree().children[0].value,
        RenderNode::FileDialog { presented: true, config }
            if config.mode == FileDialogMode::Save
                && config.file_name.as_deref() == Some("notes.txt")
    ));

    app.dispatch(
        &[0, 0],
        Event::FileDialogCompleted {
            paths: Some(vec![PathBuf::from("/tmp/notes.txt")]),
        },
    );
    assert_eq!(*chosen.borrow(), [PathBuf::from("/tmp/notes.txt")]);
    assert!(!app.get(shown));

    app.set(shown, true);
    app.dispatch(&[0, 0], Event::FileDialogCompleted { paths: None });
    assert_eq!(cancels.count(), 1);
}

#[test]
fn color_panel_writes_color_and_presentation_stores() {
    // Verifies the color panel edits its bound color and closing it clears the presentation.
    let (mut app, (color, shown)) = WithStores::mount(
        |cx| (cx.store(Color::BLACK), cx.store(true)),
        |(color, shown)| {
            Window::new("Colors")
                .size(Size::new(400.0, 300.0))
                .color_panel(ColorPanel::new().bind(color).bind_presented(shown))
        },
    );
    let red = Color::rgb(1.0, 0.0, 0.0);

    app.dispatch(&[0, 0], Event::ColorChanged { color: red });
    assert_eq!(app.get(color), red);
    assert!(matches!(
        app.tree().children[0].value,
        RenderNode::ColorPanel { color, presented: true, .. } if color == red
    ));

    app.dispatch(&[0, 0], Event::ColorPanelClosed);
    assert!(!app.get(shown));
}

#[test]
fn bound_window_closes_and_reopens_from_its_store() {
    // Verifies closing a bound panel removes it and setting the Store declares it again.
    let (mut app, open) = WithStores::mount(
        |cx| cx.store(true),
        |open| {
            Window::panel("Inspector")
                .size(Size::new(200.0, 200.0))
                .bind_open(open)
        },
    );
    assert!(matches!(
        app.tree().value,
        RenderNode::Window {
            style: WindowStyle::Panel,
            ..
        }
    ));

    assert!(app.dispatch(&[0], Event::WindowDidClose));
    assert!(!app.get(open));
    assert!(app.roots().is_empty());

    app.set(open, true);
    assert_eq!(app.roots().len(), 1);
}

#[test]
fn main_menu_orders_standard_and_custom_menus() {
    // Verifies the macOS menu order, standard titles, extensions, and custom item routing.
    let runs = Calls::default();
    let run = runs.clone();
    let (mut app, ()) = WithStores::mount(
        |_| (),
        move |()| {
            list![
                MainMenu::standard()
                    .app_name("Demo")
                    .extend(StandardMenu::File, [MenuItem::new("New")])
                    .menu(Menu::new(
                        "Tools",
                        [MenuItem::new("Run").on_select(run.action())]
                    )),
                Window::new("Demo").size(Size::new(200.0, 200.0)),
            ]
        },
    );
    let roots = app.roots();
    let RenderNode::MainMenu(commands) = &roots[0].value else {
        panic!("the main menu renders as a root node");
    };
    let titles = commands
        .menus
        .iter()
        .map(|menu| menu.title.as_str())
        .collect::<Vec<_>>();
    assert_eq!(titles, ["File", "Edit", "View", "Tools", "Window", "Help"]);
    let app_items = &commands.app_menu.as_ref().unwrap().items;
    assert_eq!(app_items[0].title, "About Demo");
    assert_eq!(app_items.last().unwrap().title, "Quit Demo");
    assert!(matches!(
        &app_items[2].submenu,
        Some(services) if services.role == Some(CommandMenuRole::Services)
    ));
    assert_eq!(commands.menus[0].items[0].title, "New");
    assert_eq!(commands.menus[4].role, Some(CommandMenuRole::Windows));
    assert_eq!(commands.menus[5].role, Some(CommandMenuRole::Help));

    assert!(app.dispatch(&[0], Event::MenuItemSelected { id: "m4.0".into() }));
    assert_eq!(runs.count(), 1);
}

#[test]
fn menu_localizers_translate_standard_text() {
    // Verifies standard titles come from the localizer and catalog keys stay unique.
    let roots = Harness::new(list![
        MainMenu::standard()
            .app_name("Demo")
            .localizer(JapaneseMenuLocalizer)
            .settings(|_| {}),
        Window::new("Demo").size(Size::new(200.0, 200.0)),
    ])
    .roots();
    let RenderNode::MainMenu(commands) = &roots[0].value else {
        panic!("the main menu renders as a root node");
    };
    let app_items = &commands.app_menu.as_ref().unwrap().items;
    assert_eq!(app_items[0].title, "Demoについて");
    assert!(matches!(
        &app_items[2],
        CommandItem { title, key: Some(_), .. } if title == "設定…"
    ));
    assert_eq!(commands.menus[1].title, "編集");

    let texts = [
        StandardText::About,
        StandardText::Settings,
        StandardText::Copy,
        StandardText::Paste,
        StandardText::WindowMenu,
        StandardText::Help,
    ];
    let keys = texts
        .iter()
        .map(|text| text.key())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(keys.len(), texts.len());
}

#[test]
fn standard_edit_menu_ends_with_a_find_submenu() {
    // Verifies the Edit menu offers text finder actions with macOS shortcuts and translated titles.
    let roots = Harness::new(list![
        MainMenu::standard().localizer(JapaneseMenuLocalizer),
        Window::new("Demo").size(Size::new(200.0, 200.0)),
    ])
    .roots();
    let RenderNode::MainMenu(commands) = &roots[0].value else {
        panic!("the main menu renders as a root node");
    };
    let find = commands.menus[1]
        .items
        .last()
        .and_then(|item| item.submenu.as_ref())
        .expect("the Edit menu ends with the Find submenu");
    let items = find
        .items
        .iter()
        .map(|item| {
            let key = match &item.key {
                Some(ShortcutKey::Character(key)) => Some(key.as_str()),
                _ => None,
            };
            (item.title.as_str(), item.role, key)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        items,
        [
            ("検索…", Some(CommandRole::Find), Some("f")),
            ("検索と置換…", Some(CommandRole::FindAndReplace), Some("f")),
            ("次を検索", Some(CommandRole::FindNext), Some("g")),
            ("前を検索", Some(CommandRole::FindPrevious), Some("g")),
            (
                "選択部分を検索に使用",
                Some(CommandRole::UseSelectionForFind),
                Some("e")
            ),
        ]
    );
}

#[test]
fn sidebar_search_updates_its_query_store() {
    // Verifies a searchable sidebar renders its query and writes native edits back.
    let (mut app, query) = WithStores::mount(
        |cx| cx.store(String::new()),
        |query| {
            Window::new("Search").size(Size::new(400.0, 300.0)).sidebar(
                Sidebar::new([SidebarSection::new([SidebarItem::new("home", "Home")])])
                    .searchable(query)
                    .search_placeholder("Search"),
            )
        },
    );

    assert!(app.dispatch(&[0], Event::SidebarSearchChanged { value: "ho".into() }));

    assert_eq!(app.get(query), "ho");
    assert!(matches!(
        app.tree().value,
        RenderNode::Window { sidebar: Some(SidebarDescription { search: Some(search), .. }), .. }
            if search.value == "ho" && search.placeholder.as_deref() == Some("Search")
    ));
}

#[test]
fn tab_view_hosts_one_page_per_tab() {
    // Verifies each tab renders a page container and native selection writes the Store.
    let (mut app, selected) = WithStores::mount(
        |cx| cx.store(0_usize),
        |selected| {
            TabView::new([
                Tab::new(0, "General", Text::new("General page")),
                Tab::new(1, "Advanced", Text::new("Advanced page")),
            ])
            .bind(selected)
        },
    );
    let tree = app.tree();
    assert!(matches!(
        &tree.value,
        RenderNode::View { kind: ViewKind::TabView { titles, selected: 0, .. }, .. } if titles == &["General", "Advanced"]
    ));
    assert_eq!(tree.children.len(), 2);
    assert!(tree.children.iter().all(|page| matches!(
        page.value,
        RenderNode::View {
            kind: ViewKind::Container,
            ..
        }
    ) && page.children.len() == 1));

    assert!(app.dispatch(&[0], Event::SelectionChanged { index: 1 }));
    assert_eq!(app.get(selected), 1);
}

#[test]
fn scroll_view_fills_its_parent_and_hosts_content() {
    // Verifies scroll views grow into free space and keep their content as a child.
    let tree = Harness::new(ScrollView::new(VStack::new(list![Text::new("Row")]))).tree();
    assert!(matches!(
        tree.value,
        RenderNode::View {
            kind: ViewKind::ScrollView { .. },
            layout: LayoutStyle {
                width: LayoutDimension::Percent(1.0),
                grow: 1.0,
                ..
            },
            ..
        }
    ));
    assert_eq!(tree.children.len(), 1);
}

#[test]
fn scroll_view_binding_saves_and_restores_offset() {
    // Verifies native scrolling updates the bound status, which the node carries for restoring.
    let (mut app, status) = WithStores::mount(
        |cx| cx.store(ScrollStatus::default()),
        |status| ScrollView::new(VStack::new(list![Text::new("Row")])).bind_offset(status),
    );
    assert!(app.dispatch(&[0], Event::Scrolled { x: 0.0, y: 120.0 }));
    assert_eq!(app.get(status), ScrollStatus { x: 0.0, y: 120.0 });
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::ScrollView {
                offset: Some((0.0, 120.0)),
                ..
            },
            ..
        }
    ));
}

/// Draws a bar `width` points wide and counts hover entries in its own state.
#[derive(Clone)]
struct Meter {
    width: f64,
    entered: u32,
}

impl Canvas2DHandler for Meter {
    fn draw(&mut self, g: &mut Graphics2D<'_>, _frame: &Frame) {
        g.fill_rect(
            Rect::new(0.0, 0.0, self.width, f64::from(self.entered)),
            Color::BLACK,
        );
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        if let CanvasEvent::Pointer(PointerEvent {
            phase: PointerPhase::Entered,
            ..
        }) = event
        {
            self.entered += 1;
            cx.invalidate();
        }
        true
    }

    fn reconcile(&mut self, next: Self) -> Redraw {
        self.width = next.width;
        Redraw::Yes
    }
}

fn canvas_node(node: &RenderNode) -> CanvasNode {
    match node {
        RenderNode::View {
            kind: ViewKind::Canvas2D { canvas, .. },
            ..
        } => canvas.clone(),
        _ => panic!("expected a canvas"),
    }
}

fn canvas_frame() -> Frame {
    Frame {
        bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
        safe_area: Rect::new(0.0, 0.0, 100.0, 100.0),
        scale: 2.0,
        time: 0.0,
        delta: 0.0,
        palette: Default::default(),
    }
}

/// Renders a meter whose width comes from a Store it publishes to the test.
struct MeterView {
    width: Store<f64>,
    published: Rc<Cell<Option<Store<f64>>>>,
}

impl MeterView {
    fn mount() -> (Harness, Store<f64>) {
        let published = Rc::new(Cell::new(None));
        let app = Harness::new(Self {
            width: Store::default(),
            published: published.clone(),
        });
        (app, published.get().expect("mounting runs init"))
    }
}

impl View for MeterView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.width = cx.store(10.0);
        self.published.set(Some(self.width));
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        cx.watch(self.width, |width| {
            Canvas2D::new(Meter {
                width: *width,
                entered: 0,
            })
            .redraw(RedrawPolicy::Continuous {
                preferred_fps: Some(60.0),
            })
            .focusable(true)
            .fill_width()
        })
    }
}

#[test]
fn canvas_declares_redraw_focus_and_layout() {
    // Verifies the builder records its redraw policy, focusability, and layout on the node.
    let (app, _) = MeterView::mount();
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::Canvas2D {
                redraw: RedrawPolicy::Continuous {
                    preferred_fps: Some(60.0)
                },
                focusable: true,
                ..
            },
            layout: LayoutStyle {
                width: LayoutDimension::Percent(1.0),
                ..
            },
            ..
        }
    ));
}

#[test]
fn canvas_keeps_handler_state_across_store_updates() {
    // Verifies a Store-driven re-render updates the width while handler-owned hover state survives.
    let (mut app, width) = MeterView::mount();
    let mounted = canvas_node(&app.tree().value);
    mounted.event(&CanvasEvent::Pointer(PointerEvent {
        phase: PointerPhase::Entered,
        button: None,
        location: Point::new(5.0, 5.0),
        modifiers: Modifiers::default(),
        click_count: 0,
    }));

    app.set(width, 40.0);

    let current = canvas_node(&app.tree().value);
    assert_eq!(current, mounted);
    assert_eq!(
        current.record(&canvas_frame()),
        [graphics::DrawCommand::Fill {
            path: Path::rect(Rect::new(0.0, 0.0, 40.0, 1.0)),
            paint: Paint::Solid(Color::BLACK),
            rule: FillRule::NonZero,
        }]
    );
}

/// Consumes input; `color` comes from a Store.
#[cfg(feature = "wgpu")]
#[derive(Clone)]
struct Scene {
    color: f64,
}

#[cfg(feature = "wgpu")]
impl WgpuCanvasHandler for Scene {
    fn render(&mut self, _target: &mut WgpuTarget<'_>, _frame: &Frame) {}

    fn event(&mut self, _event: &CanvasEvent, _cx: &mut CanvasEventContext) -> bool {
        true
    }

    fn reconcile(&mut self, next: Self) -> Redraw {
        self.color = next.color;
        Redraw::Yes
    }
}

/// Renders a wgpu canvas whose color comes from a Store it publishes to the test.
#[cfg(feature = "wgpu")]
struct SceneView {
    color: Store<f64>,
    entered: Rc<Cell<u32>>,
    published: Rc<Cell<Option<Store<f64>>>>,
}

#[cfg(feature = "wgpu")]
impl View for SceneView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.color = cx.store(0.0);
        self.published.set(Some(self.color));
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let entered = self.entered.clone();
        cx.watch(self.color, move |color| {
            let entered = entered.clone();
            WgpuCanvas::new(Scene { color: *color })
                .redraw(RedrawPolicy::Continuous {
                    preferred_fps: None,
                })
                .focusable(true)
                .grow()
                .on_event(move |_, _| entered.set(entered.get() + 1))
        })
    }
}

#[cfg(feature = "wgpu")]
#[test]
fn wgpu_canvas_declares_redraw_focus_and_layout() {
    // Verifies the wgpu builder records its redraw policy, focusability, and layout like Canvas2D.
    let published = Rc::new(Cell::new(None));
    let app = Harness::new(SceneView {
        color: Store::default(),
        entered: Rc::default(),
        published,
    });
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::WgpuCanvas {
                redraw: RedrawPolicy::Continuous {
                    preferred_fps: None
                },
                focusable: true,
                ..
            },
            layout: LayoutStyle { grow: 1.0, .. },
            ..
        }
    ));
}

#[cfg(feature = "wgpu")]
#[test]
fn wgpu_canvas_keeps_its_handler_and_routes_events() {
    // Verifies a Store-driven re-render keeps the mounted wgpu handler and on_event sees its input.
    let published = Rc::new(Cell::new(None));
    let entered = Rc::new(Cell::new(0));
    let mut app = Harness::new(SceneView {
        color: Store::default(),
        entered: entered.clone(),
        published: published.clone(),
    });
    let color = published.get().expect("mounting runs init");
    let node = |app: &Harness| match app.tree().value {
        RenderNode::View {
            kind: ViewKind::WgpuCanvas { canvas, .. },
            ..
        } => canvas,
        _ => panic!("expected a wgpu canvas"),
    };
    let mounted = node(&app);
    let pointer = CanvasEvent::Pointer(PointerEvent {
        phase: PointerPhase::Entered,
        button: None,
        location: Point::new(1.0, 1.0),
        modifiers: Modifiers::default(),
        click_count: 0,
    });
    assert!(mounted.event(&pointer));
    app.dispatch(&[0], Event::Canvas(pointer));
    assert_eq!(entered.get(), 1);

    app.set(color, 0.5);
    assert_eq!(node(&app), mounted);
}

/// Returns a primary-button pointer event at `x` along a horizontal split view.
fn divider_pointer(phase: PointerPhase, x: f64) -> CanvasEvent {
    CanvasEvent::Pointer(PointerEvent {
        phase,
        button: Some(PointerButton::Primary),
        location: graphics::Point::new(x, 10.0),
        modifiers: Modifiers::default(),
        click_count: 1,
    })
}

/// Returns the first pane's declared width.
fn first_pane_width(app: &Harness) -> LayoutDimension {
    match &app.tree().children[0].value {
        RenderNode::View { layout, .. } => layout.width,
        other => panic!("expected the first pane, got {other:?}"),
    }
}

#[test]
fn split_view_divider_drag_resizes_the_first_pane_within_its_range() {
    // Verifies dragging the divider moves it by the pointer's distance from the grab point and clamps it.
    let (mut app, width) = WithStores::mount(
        |cx| cx.store(200.0_f32),
        |width| {
            SplitView::horizontal(Text::new("List"), Text::new("Detail"))
                .range(100.0..=300.0)
                .bind(width)
        },
    );

    // The divider is the second child of the root stack, and it consumes the drag.
    assert!(app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Down, 3.0)));
    assert!(app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Moved, 23.0)));
    assert_eq!(app.get(width), 220.0);
    assert_eq!(first_pane_width(&app), LayoutDimension::Points(220.0));

    app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Moved, 503.0));
    assert_eq!(app.get(width), 300.0);

    // Moves after the button is released do not resize.
    app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Up, 503.0));
    app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Moved, 0.0));
    assert_eq!(app.get(width), 300.0);
}

#[test]
fn split_view_follows_a_drag_while_its_parent_reads_the_width() {
    // Verifies two moves move the divider twice when each move re-renders the parent that reads the bound width.
    let (mut app, width) = WithStores::mount(|cx| cx.store(200.0_f32), |width| WatchWidth(width));
    let divider = |app: &Harness| match &app.tree().children[1].value {
        RenderNode::View {
            kind: ViewKind::Canvas2D { canvas, .. },
            ..
        } => canvas.clone(),
        other => panic!("expected the divider, got {other:?}"),
    };
    let mounted = divider(&app);

    app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Down, 3.0));
    app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Moved, 23.0));
    // The divider moved with the pointer, so another 20 points lands at the same canvas location.
    app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Moved, 23.0));

    assert_eq!(app.get(width), 240.0);
    // The re-renders reconcile into the mounted divider instead of replacing it.
    assert_eq!(divider(&app), mounted);
}

/// Reads the bound width while rendering, so every width change re-renders it.
struct WatchWidth(Store<f32>);

impl View for WatchWidth {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let _ = cx.get(self.0);
        SplitView::horizontal(Text::new("List"), Text::new("Detail")).bind(self.0)
    }
}

#[test]
fn split_view_clamps_its_position_to_a_range_declared_later() {
    // Verifies the range clamps the starting size even when it is declared after the position.
    let app = Harness::new(
        SplitView::horizontal(Text::new("List"), Text::new("Detail"))
            .position(500.0)
            .range(100.0..=300.0),
    );

    assert_eq!(first_pane_width(&app), LayoutDimension::Points(300.0));
}

#[test]
fn split_view_drag_survives_a_rebuild_of_its_parent() {
    // Verifies a drag in progress keeps its grab point when the parent renders a new split view.
    let (mut app, (width, version)) = WithStores::mount(
        |cx| (cx.store(200.0_f32), cx.store(0_u32)),
        |(width, version)| {
            let render = move |_: &u32| {
                SplitView::horizontal(Text::new("List"), Text::new("Detail")).bind(width)
            };
            WatchVersion(version, Rc::new(render))
        },
    );

    app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Down, 3.0));
    app.update(version, |version| *version += 1);
    app.canvas_event(&[0, 1], divider_pointer(PointerPhase::Moved, 13.0));

    assert_eq!(app.get(width), 210.0);
}

/// Renders a new view each time a version Store changes.
#[derive(Clone)]
struct WatchVersion<E>(Store<u32>, Rc<dyn Fn(&u32) -> E>);

impl<E: IntoElement<Output = RenderNode> + 'static> View for WatchVersion<E> {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        (self.1)(cx.get(self.0))
    }
}

#[test]
fn split_view_starts_at_its_position_inside_the_range() {
    // Verifies an unbound split view starts at its declared size, clamped to the range.
    let app = Harness::new(
        SplitView::vertical(Text::new("Top"), Text::new("Bottom"))
            .position(40.0)
            .range(80.0..=200.0),
    );

    let RenderNode::View { layout, .. } = &app.tree().children[0].value else {
        panic!("expected the first pane");
    };
    assert_eq!(layout.height, LayoutDimension::Points(80.0));
}

#[test]
fn list_describes_sections_and_writes_user_selection() {
    // Verifies a list renders its sections and selection, one child per row, and reports user picks.
    let picked = Rc::new(RefCell::new(Vec::new()));
    let recorded = picked.clone();
    let (mut app, selection) = WithStores::mount(
        |cx| cx.store(Some("b")),
        move |selection| {
            let recorded = recorded.clone();
            List::new([
                ListSection::new([ListRow::new("a", Text::new("A"))]).title("Pinned"),
                ListSection::new([
                    ListRow::new("b", Text::new("B")),
                    ListRow::new("c", Text::new("C")),
                ]),
            ])
            .bind(selection)
            .on_select(move |_, id| recorded.borrow_mut().push(id.copied()))
        },
    );

    let tree = app.tree();
    let RenderNode::View {
        kind: ViewKind::List {
            sections, selected, ..
        },
        ..
    } = &tree.value
    else {
        panic!("expected a list, got {:?}", tree.value);
    };
    assert_eq!(sections[0].title.as_deref(), Some("Pinned"));
    assert_eq!(sections[1].ids, ["b", "c"]);
    assert_eq!(selected.as_deref(), Some("b"));
    assert_eq!(tree.children.len(), 3);

    assert!(app.dispatch(
        &[0],
        Event::ListSelectionChanged {
            id: Some("c".into())
        }
    ));
    assert_eq!(app.get(selection), Some("c"));
    assert!(app.dispatch(&[0], Event::ListSelectionChanged { id: None }));
    assert_eq!(app.get(selection), None);
    assert_eq!(*picked.borrow(), [Some("c"), None]);
}

#[test]
fn text_field_updates_its_store_before_on_change() {
    // Verifies a field writes the bound Store, then passes the new text to `on_change`.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let recorded = seen.clone();
    let (mut app, _) = WithStores::mount(
        |cx| cx.store(String::new()),
        move |title| {
            let recorded = recorded.clone();
            TextField::new().bind(title).on_change(move |cx, text| {
                recorded
                    .borrow_mut()
                    .push((cx.get(title).clone(), text.to_owned()))
            })
        },
    );

    app.dispatch(&[0], Event::TextChanged { value: "T".into() });

    assert_eq!(*seen.borrow(), [("T".to_owned(), "T".to_owned())]);
}

#[test]
fn text_editor_updates_its_store_before_on_change() {
    // Verifies the editor renders styled text and stores native edits before `on_change` runs.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let recorded = seen.clone();
    let (mut app, body) = WithStores::mount(
        |cx| {
            let mut text = AttributedText::from("Title");
            text.set_paragraph(0, ParagraphStyle::Title);
            cx.store(text)
        },
        move |body| {
            let recorded = recorded.clone();
            TextEditor::new().bind(body).on_change(move |cx, text| {
                // The Store already holds the text the action receives.
                assert_eq!(cx.get(body), text);
                recorded.borrow_mut().push(text.text().to_owned())
            })
        },
    );

    assert!(matches!(
        &app.tree().value,
        RenderNode::View { kind: ViewKind::TextEditor { text, .. }, .. }
            if text.paragraphs() == [ParagraphStyle::Title]
    ));
    let mut edited = AttributedText::from("Title\n");
    edited.set_paragraph(0, ParagraphStyle::Title);
    edited.push(
        "bold",
        SpanStyle {
            bold: true,
            ..SpanStyle::default()
        },
    );
    assert!(app.dispatch(
        &[0],
        Event::RichTextChanged {
            text: edited.clone()
        }
    ));

    assert_eq!(app.get(body), edited);
    assert_eq!(*seen.borrow(), ["Title\nbold"]);
}

#[test]
fn toolbar_search_carries_its_id_and_focus_requests() {
    // Verifies the search item declares its ID and counts the fires of its focus trigger.
    let (mut app, (focus, _)) = WithStores::mount(
        |cx| (cx.trigger(), cx.store(String::new())),
        |(focus, query)| {
            Window::new("Toolbar")
                .size(Size::new(400.0, 300.0))
                .toolbar(Toolbar::new([ToolbarItem::search(query)
                    .id("toolbar-search")
                    .focus_on(focus)]))
        },
    );
    let search = |app: &Harness| match &app.tree().value {
        RenderNode::Window {
            toolbar: Some(toolbar),
            ..
        } => toolbar.items[0].clone(),
        _ => panic!("expected a window with a toolbar"),
    };
    assert!(matches!(
        search(&app),
        zintl_ui_backend_core::ToolbarItem::Search { id: Some(id), focus: 0, .. } if id == "toolbar-search"
    ));
    app.fire(focus, ());
    assert!(matches!(
        search(&app),
        zintl_ui_backend_core::ToolbarItem::Search { focus: 1, .. }
    ));
}

#[test]
fn toolbar_describes_items_and_routes_clicks_and_search() {
    // Verifies toolbar items render with state, buttons run their actions, and search writes its Store.
    let clicks = Calls::default();
    let click = clicks.clone();
    let (mut app, query) = WithStores::mount(
        |cx| cx.store(String::new()),
        move |query| {
            Window::new("Toolbar")
                .size(Size::new(400.0, 300.0))
                .toolbar(Toolbar::new([
                    ToolbarItem::button("new", "New", "plus").on_click(click.action()),
                    ToolbarItem::flexible_space(),
                    ToolbarItem::search(query).placeholder("Search"),
                ]))
        },
    );

    let RenderNode::Window {
        toolbar: Some(toolbar),
        ..
    } = &app.tree().value
    else {
        panic!("expected a window with a toolbar");
    };
    assert_eq!(toolbar.items.len(), 3);
    assert!(matches!(
        &toolbar.items[2],
        zintl_ui_backend_core::ToolbarItem::Search { placeholder: Some(text), .. } if text == "Search"
    ));

    assert!(app.dispatch(&[0], Event::ToolbarItemClicked { id: "new".into() }));
    assert!(app.dispatch(
        &[0],
        Event::ToolbarSearchChanged {
            value: "roux".into()
        }
    ));
    assert_eq!(clicks.count(), 1);
    assert_eq!(app.get(query), "roux");
}

#[test]
fn sidebar_items_show_badges_and_route_their_menus() {
    // Verifies item badges render and item menu selections run actions through the window.
    let renames = Calls::default();
    let rename = renames.clone();
    let (mut app, ()) = WithStores::mount(
        |_| (),
        move |()| {
            Window::new("Folders")
                .size(Size::new(400.0, 300.0))
                .sidebar(Sidebar::new([SidebarSection::new([SidebarItem::new(
                    "ideas", "Ideas",
                )
                .badge("3")
                .context_menu([
                    MenuItem::new("Rename").on_select(rename.action())
                ])])]))
        },
    );

    let RenderNode::Window {
        sidebar: Some(sidebar),
        ..
    } = &app.tree().value
    else {
        panic!("expected a window with a sidebar");
    };
    let item = &sidebar.sections[0].items[0];
    assert_eq!(item.badge.as_deref(), Some("3"));
    assert_eq!(item.menu.len(), 1);
    let id = item.menu[0].id.clone().expect("custom items have IDs");

    assert!(app.dispatch(&[0], Event::MenuItemSelected { id }));
    assert_eq!(renames.count(), 1);
}

/// Counts close requests, undos, and redos; closing is allowed once `allow` is set.
struct DocumentWindow {
    allow: Store<bool>,
    requests: Store<u32>,
    steps: Store<i32>,
    published: Rc<Cell<Option<(Store<bool>, Store<u32>, Store<i32>)>>>,
}

impl DocumentWindow {
    fn mount() -> (Harness, Store<bool>, Store<u32>, Store<i32>) {
        let published = Rc::new(Cell::new(None));
        let app = Harness::new(Self {
            allow: Store::default(),
            requests: Store::default(),
            steps: Store::default(),
            published: published.clone(),
        });
        let (allow, requests, steps) = published.get().expect("mounting runs init");
        (app, allow, requests, steps)
    }
}

impl View for DocumentWindow {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.allow = cx.store(false);
        self.requests = cx.store(0);
        self.steps = cx.store(0);
        self.published
            .set(Some((self.allow, self.requests, self.steps)));
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (allow, requests, steps) = (self.allow, self.requests, self.steps);
        let count = *cx.get(steps);
        Window::new("Document")
            .size(Size::new(300.0, 200.0))
            .edited(count != 0)
            .on_close_request(move |cx| {
                cx.update(requests, |requests| *requests += 1);
                *cx.get(allow)
            })
            .undo(
                UndoHandler::new(count > 0, count < 0)
                    .on_undo(move |cx| cx.update(steps, |steps| *steps -= 1))
                    .on_redo(move |cx| cx.update(steps, |steps| *steps += 1)),
            )
    }
}

fn document(app: &Harness) -> DocumentState {
    match app.tree().value {
        RenderNode::Window { document, .. } => document,
        other => panic!("expected a window, got {other:?}"),
    }
}

#[test]
fn close_request_keeps_the_window_until_the_handler_allows_it() {
    // Verifies a refused request leaves the close count alone and an allowed one asks the backend to close.
    let (mut app, allow, requests, _) = DocumentWindow::mount();
    assert!(document(&app).confirms_close);

    app.dispatch(&[0], Event::WindowCloseRequested);
    assert_eq!(app.get(requests), 1);
    assert_eq!(document(&app).close_requests, 0);

    app.set(allow, true);
    app.dispatch(&[0], Event::WindowCloseRequested);
    assert_eq!(document(&app).close_requests, 1);
}

#[test]
fn undo_events_run_the_window_history_and_update_menu_state() {
    // Verifies Undo and Redo reach the handler and the window reports its edited and enabled states.
    let (mut app, _, _, steps) = DocumentWindow::mount();
    assert_eq!(
        document(&app).undo,
        Some(UndoState {
            can_undo: false,
            can_redo: false
        })
    );

    app.dispatch(&[0], Event::Redo);
    assert_eq!(app.get(steps), 1);
    let state = document(&app);
    assert!(state.edited);
    assert_eq!(state.undo.map(|undo| undo.can_undo), Some(true));

    app.dispatch(&[0], Event::Undo);
    assert_eq!(app.get(steps), 0);
    assert!(!document(&app).edited);
}

#[test]
fn help_text_reaches_any_view() {
    // Verifies the help modifier sets the tooltip on controls and containers alike.
    let app = Harness::new(
        VStack::new(list![
            Button::new("Save").enabled(false).help("Nothing to save")
        ])
        .help("Tools"),
    );
    let tree = app.tree();
    assert!(matches!(
        &tree.value,
        RenderNode::View { help: Some(help), .. } if help == "Tools"
    ));
    assert!(matches!(
        &tree.children[0].value,
        RenderNode::View { help: Some(help), enabled: false, .. } if help == "Nothing to save"
    ));
}

#[test]
fn menu_button_shows_its_menu_instead_of_activating() {
    // Verifies a menu button marks itself for the native pull-down and carries the menu as an attachment.
    let app = Harness::new(Button::new("Samples").menu(Menu::items([MenuItem::new("Load A")])));
    let tree = app.tree();
    assert!(matches!(
        &tree.value,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                shows_menu: true,
                ..
            }),
            ..
        }
    ));
    assert!(matches!(
        &tree.children[..],
        [child] if matches!(child.value, RenderNode::ContextMenu { .. })
    ));
}

#[test]
fn selected_button_draws_in_the_on_state() {
    // Verifies a selected button is a push-on-push-off button that shows its selection.
    let app = Harness::new(Button::new("1").selected(true));
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                button_type: ButtonType::PushOnPushOff,
                on: true,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn selected_button_runs_on_click_and_keeps_its_state() {
    // Verifies a click on a selected button runs `on_click` and leaves the drawn state to the view.
    let clicks = Calls::default();
    let mut app = Harness::new(Button::new("1").selected(false).on_click(clicks.action()));
    app.tree().value;

    assert!(app.dispatch(&[0], Event::Activated));

    assert_eq!(clicks.count(), 1);
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::Button(zintl_ui_backend_core::Button {
                on: false,
                keeps_state: true,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn indicators_describe_their_kinds() {
    // Verifies the spinner has no value, the bar keeps its fraction, and separators are thin lines.
    let render = |view: Element<RenderNode>| Harness::new(view).tree().value;
    assert!(matches!(
        render(ProgressIndicator::spinning().into_element()),
        RenderNode::View {
            kind: ViewKind::ProgressIndicator { value: None },
            ..
        }
    ));
    assert!(matches!(
        render(ProgressIndicator::bar(0.25).into_element()),
        RenderNode::View {
            kind: ViewKind::ProgressIndicator { value: Some(0.25) },
            ..
        }
    ));
    assert!(matches!(
        render(Separator::vertical().into_element()),
        RenderNode::View {
            kind: ViewKind::Separator,
            layout: LayoutStyle {
                width: LayoutDimension::Points(1.0),
                ..
            },
            ..
        }
    ));
}

#[test]
fn zstack_and_grid_declare_their_arrangements() {
    // Verifies ZStack overlays its children and a striped Grid adds one stripe per shaded row before its cells.
    let zstack = Harness::new(ZStack::new(list![
        Text::new("Scene"),
        Text::new("Updating")
    ]));
    assert!(matches!(
        zstack.tree().value,
        RenderNode::View {
            layout: LayoutStyle {
                arrangement: zintl_ui_layout::Arrangement::Overlay,
                ..
            },
            ..
        }
    ));

    let rows = (0..3).map(|row| {
        GridRow::new(list![
            Text::new(format!("{row}a")),
            Text::new(format!("{row}b"))
        ])
    });
    // Three rows of two cells; only the middle one is shaded.
    let grid = Harness::new(Grid::new(rows).striped(true)).tree();
    assert!(matches!(
        grid.value,
        RenderNode::View {
            layout: LayoutStyle {
                arrangement: zintl_ui_layout::Arrangement::Grid { columns: 2 },
                ..
            },
            ..
        }
    ));
    assert_eq!(grid.children.len(), 7);
    assert!(matches!(
        grid.children[0].value,
        RenderNode::View {
            layout: LayoutStyle {
                grid_cell: Some(zintl_ui_layout::GridCell {
                    row: 1,
                    column: 0,
                    column_span: 0
                }),
                ..
            },
            ..
        }
    ));
}

#[test]
fn grid_rows_stripe_the_right_rows_around_a_spanning_heading() {
    // Verifies stripes land on the second and fourth rows around a spanning heading, and short rows are filled so the next row starts in the first column.
    let grid = Harness::new(
        Grid::new([
            GridRow::new(list![Text::new("a"), Text::new("b")]),
            GridRow::new(list![Text::new("Heading")]).span(0, 3),
            GridRow::new(list![Text::new("c"), Text::new("d"), Text::new("e")]),
            GridRow::new(list![Text::new("f")]),
        ])
        .striped(true),
    )
    .tree();
    let stripe_rows = grid
        .children
        .iter()
        .filter_map(|child| match &child.value {
            RenderNode::View {
                layout:
                    LayoutStyle {
                        grid_cell: Some(cell),
                        ..
                    },
                ..
            } => Some(cell.row),
            _ => None,
        })
        .collect::<Vec<_>>();
    let flowing = grid
        .children
        .iter()
        .filter_map(|child| match &child.value {
            RenderNode::View { layout, .. } if layout.grid_cell.is_none() => {
                Some(layout.column_span)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(matches!(
        grid.value,
        RenderNode::View {
            layout: LayoutStyle {
                arrangement: zintl_ui_layout::Arrangement::Grid { columns: 3 },
                ..
            },
            ..
        }
    ));
    assert_eq!(stripe_rows, [1, 3]);
    // Each row fills all three columns, so every row starts in the first column.
    assert_eq!(flowing, [1, 1, 1, 3, 1, 1, 1, 1, 1, 1]);
}

#[test]
fn disclosure_group_shows_content_only_while_expanded() {
    // Verifies the disclosure button's state writes the bound Store and the content follows it.
    let (mut app, expanded) = WithStores::mount(
        |cx| cx.store(false),
        |expanded| DisclosureGroup::new("Line settings", Text::new("Details")).bind(expanded),
    );
    assert_eq!(app.tree().children.len(), 1);

    // The disclosure button is the first child of the header row.
    app.dispatch(&[0, 0, 0], Event::StateChanged { on: true });
    assert!(app.get(expanded));
    assert_eq!(app.tree().children.len(), 2);
}

#[test]
fn scroll_view_reveals_the_target_row() {
    // Verifies a fired trigger becomes the scroll view's reveal request with its alignment.
    let (mut app, target) = WithStores::mount(
        |cx| cx.trigger::<ViewId>(),
        |target| {
            ScrollView::new(VStack::new(list![Text::new("Row").id("row-1")]))
                .scroll_to(target, ScrollAlignment::Center)
        },
    );
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::ScrollView { reveal: None, .. },
            ..
        }
    ));

    app.fire(target, "row-1".into());
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::ScrollView {
                reveal: Some(ScrollTarget { id, alignment: ScrollAlignment::Center, .. }),
                ..
            },
            ..
        } if id == "row-1"
    ));
}

#[test]
fn scroll_view_reveals_the_same_row_again_when_its_trigger_fires_again() {
    // Verifies firing the same ID again makes a new reveal request, while plain rerenders do not.
    let (mut app, target) = WithStores::mount(
        |cx| cx.trigger::<ViewId>(),
        |target| {
            ScrollView::new(VStack::new(list![Text::new("Row").id("row-1")]))
                .scroll_to(target, ScrollAlignment::Nearest)
        },
    );
    let reveal = |app: &Harness| match app.tree().value {
        RenderNode::View {
            kind: ViewKind::ScrollView { reveal, .. },
            ..
        } => reveal,
        _ => panic!("expected a scroll view"),
    };
    app.fire(target, "row-1".into());
    let first = reveal(&app).expect("the row is requested");
    assert_eq!(reveal(&app), Some(first.clone()));

    app.fire(target, "row-1".into());
    let second = reveal(&app).expect("the row is requested again");
    assert_eq!(second.id, first.id);
    assert_ne!(second.revision, first.revision);
}

#[test]
fn table_height_fixes_its_layout_height() {
    // Verifies a table with a height stops growing and uses that height.
    let (app, _) = WithStores::mount(
        |_| (),
        |()| Table::<String>::new([TableColumn::new("Name")], []).height(180.0),
    );
    assert!(matches!(
        app.tree().value,
        RenderNode::View {
            kind: ViewKind::Table { .. },
            layout,
            ..
        } if layout.height == LayoutDimension::Points(180.0) && layout.grow == 0.0
    ));
}

#[test]
fn unbound_fields_show_their_values() {
    // Verifies `value` sets what unbound text and number fields show.
    let (app, _) = WithStores::mount(
        |_| (),
        |()| {
            HStack::new(list![
                TextField::new().value("cell"),
                NumberField::new().value(2.5)
            ])
        },
    );
    let tree = app.tree();
    assert!(matches!(
        &tree.children[0].value,
        RenderNode::View { kind: ViewKind::TextField { value, .. }, .. } if value == "cell"
    ));
    assert!(matches!(
        &tree.children[1].value,
        RenderNode::View { kind: ViewKind::NumberField { value, .. }, .. } if *value == 2.5
    ));
}

#[test]
fn bound_fields_prefer_their_stores_over_values() {
    // Verifies a bound Store wins over `value` on both field kinds.
    let (app, _) = WithStores::mount(
        |cx| (cx.store("stored".to_owned()), cx.store(7.0_f64)),
        |(text, number)| {
            HStack::new(list![
                TextField::new().value("ignored").bind(text),
                NumberField::new().value(1.0).bind(number),
            ])
        },
    );
    let tree = app.tree();
    assert!(matches!(
        &tree.children[0].value,
        RenderNode::View { kind: ViewKind::TextField { value, .. }, .. } if value == "stored"
    ));
    assert!(matches!(
        &tree.children[1].value,
        RenderNode::View { kind: ViewKind::NumberField { value, .. }, .. } if *value == 7.0
    ));
}

#[test]
fn table_lays_out_cells_by_row_and_writes_the_selection() {
    // Verifies a table declares its columns and rows, pads short rows to one cell per column, and binds selection.
    let (mut app, selected) = WithStores::mount(
        |cx| cx.store(None::<&str>),
        |selected| {
            Table::new(
                [
                    TableColumn::new("Name").width(120.0),
                    TableColumn::new("Length"),
                ],
                [
                    TableRow::new("a", list![Text::new("Section A"), Text::new("1k200")]),
                    TableRow::new("b", list![Text::new("Section B")]),
                ],
            )
            .bind(selected)
        },
    );
    let tree = app.tree();
    assert!(matches!(
        &tree.value,
        RenderNode::View {
            kind: ViewKind::Table { columns, rows, striped: true, selected: None, .. },
            ..
        } if columns.len() == 2 && columns[0].width == Some(120.0) && rows == &["a", "b"]
    ));
    assert_eq!(tree.children.len(), 4);

    app.dispatch(
        &[0],
        Event::ListSelectionChanged {
            id: Some("b".into()),
        },
    );
    assert_eq!(app.get(selected), Some("b"));
}

/// A row key of the app's own type, as lists, tables, and sidebars select by.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FolderId(u32);

impl ItemKey for FolderId {
    fn item_id(&self) -> String {
        format!("folder-{}", self.0)
    }
}

/// Builds a list with one text row per folder.
fn folder_list(folders: &[u32]) -> List<FolderId> {
    List::new([ListSection::new(folders.iter().map(|&number| {
        ListRow::new(FolderId(number), Text::new(format!("Folder {number}")))
    }))])
}

/// Returns the row IDs and selection a rendered list describes.
fn list_rows(node: &RenderedNode) -> (Vec<String>, Option<String>) {
    match &node.value {
        RenderNode::View {
            kind: ViewKind::List {
                sections, selected, ..
            },
            ..
        } => (
            sections
                .iter()
                .flat_map(|section| section.ids.clone())
                .collect(),
            selected.clone(),
        ),
        other => panic!("expected a list, got {other:?}"),
    }
}

#[test]
fn list_selects_rows_by_their_key() {
    // Verifies a list sends each key's item ID to the backend and shows the bound key as selected.
    let (app, _) = WithStores::mount(
        |cx| cx.store(Some(FolderId(2))),
        |selection| folder_list(&[1, 2]).bind(selection),
    );

    let (ids, selected) = list_rows(&app.tree());
    assert_eq!(ids, ["folder-1", "folder-2"]);
    assert_eq!(selected.as_deref(), Some("folder-2"));
}

#[test]
fn list_shows_no_selection_for_a_key_no_row_has() {
    // Verifies a bound key missing from the rows renders as no selection.
    let (app, _) = WithStores::mount(
        |cx| cx.store(Some(FolderId(9))),
        |selection| folder_list(&[1, 2]).bind(selection),
    );

    assert_eq!(list_rows(&app.tree()).1, None);
}

#[test]
fn list_reports_the_key_of_the_chosen_row() {
    // Verifies a reported item ID reaches the Store and `on_select` as its key, and unknown IDs are ignored.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = seen.clone();
    let (mut app, selection) = WithStores::mount(
        |cx| cx.store(None::<FolderId>),
        move |selection| {
            let record = record.clone();
            folder_list(&[1, 2])
                .bind(selection)
                .on_select(move |_, key| record.borrow_mut().push(key.copied()))
        },
    );

    app.dispatch(
        &[0],
        Event::ListSelectionChanged {
            id: Some("folder-2".into()),
        },
    );
    assert_eq!(app.get(selection), Some(FolderId(2)));
    // A row removed before its event arrives leaves everything as it was.
    app.dispatch(
        &[0],
        Event::ListSelectionChanged {
            id: Some("folder-9".into()),
        },
    );
    assert_eq!(app.get(selection), Some(FolderId(2)));
    app.dispatch(&[0], Event::ListSelectionChanged { id: None });

    assert_eq!(app.get(selection), None);
    assert_eq!(*seen.borrow(), [Some(FolderId(2)), None]);
}

/// Numbers views in the order they first mount; the number lives in a Store, so a kept view keeps it.
#[derive(Clone)]
struct Numbered {
    name: &'static str,
    mounts: Rc<Cell<u32>>,
    number: Store<u32>,
}

impl Numbered {
    fn new(name: &'static str, mounts: &Rc<Cell<u32>>) -> Self {
        Self {
            name,
            mounts: mounts.clone(),
            number: Store::default(),
        }
    }

    /// Returns the `name-number` IDs of the rendered instances, in order.
    fn ids(nodes: &[RenderedNode]) -> Vec<String> {
        nodes
            .iter()
            .map(|node| match &node.value {
                RenderNode::View { id: Some(id), .. } => id.clone(),
                other => panic!("expected a named view, got {other:?}"),
            })
            .collect()
    }
}

impl View for Numbered {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.mounts.set(self.mounts.get() + 1);
        self.number = cx.store(self.mounts.get());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Text::new(self.name).id(format!("{}-{}", self.name, cx.get(self.number)))
    }
}

#[test]
fn list_rows_keep_their_views_when_reordered() {
    // Verifies rows are keyed by item ID, so moving a row moves its view instead of remounting it.
    let mounts = Rc::new(Cell::new(0));
    let rows = mounts.clone();
    let (mut app, reversed) = WithStores::mount(
        |cx| cx.store(false),
        move |reversed| {
            let mounts = rows.clone();
            WithReversed(reversed, move |reversed: bool| {
                let mut names = vec!["a", "b"];
                if reversed {
                    names.reverse();
                }
                List::new([ListSection::new(
                    names
                        .into_iter()
                        .map(|name| ListRow::new(name, Numbered::new(name, &mounts))),
                )])
            })
        },
    );
    assert_eq!(Numbered::ids(&app.tree().children), ["a-1", "b-2"]);

    app.set(reversed, true);

    // The numbers moved with their rows; remounted rows would show b-1 and a-2.
    assert_eq!(Numbered::ids(&app.tree().children), ["b-2", "a-1"]);
}

/// Stores a table's selection and records the keys its callbacks receive.
#[derive(Clone, Default)]
struct TableCalls {
    activated: Rc<RefCell<Vec<FolderId>>>,
    deleted: Rc<RefCell<Vec<FolderId>>>,
    checked: Rc<RefCell<Vec<Option<FolderId>>>>,
}

impl TableCalls {
    /// Builds a table of folders 1 and 2 that allows selection while `allow` is set.
    fn table(&self, selection: Store<Option<FolderId>>, allow: Store<bool>) -> Table<FolderId> {
        let (activated, deleted, checked) = (
            self.activated.clone(),
            self.deleted.clone(),
            self.checked.clone(),
        );
        Table::new(
            [TableColumn::new("Name")],
            [1, 2].map(|number| {
                TableRow::new(
                    FolderId(number),
                    list![Text::new(format!("Folder {number}"))],
                )
            }),
        )
        .bind(selection)
        .should_select(move |cx, key| {
            checked.borrow_mut().push(key.copied());
            *cx.get(allow)
        })
        .on_activate(move |_, key| activated.borrow_mut().push(*key))
        .on_delete(move |_, key| deleted.borrow_mut().push(*key))
    }
}

#[test]
fn table_passes_row_keys_to_its_callbacks() {
    // Verifies activation, deletion, and selection checks receive keys, and unknown row IDs run nothing.
    let calls = TableCalls::default();
    let table = calls.clone();
    let (mut app, (selection, allow)) = WithStores::mount(
        |cx| (cx.store(None::<FolderId>), cx.store(true)),
        move |(selection, allow)| table.table(selection, allow),
    );

    app.dispatch(
        &[0],
        Event::RowActivated {
            id: "folder-2".into(),
        },
    );
    app.dispatch(
        &[0],
        Event::RowDeleteRequested {
            id: "folder-1".into(),
        },
    );
    app.dispatch(
        &[0],
        Event::RowActivated {
            id: "folder-9".into(),
        },
    );
    app.dispatch(
        &[0],
        Event::RowDeleteRequested {
            id: "folder-9".into(),
        },
    );
    app.dispatch(
        &[0],
        Event::ListSelectionChanged {
            id: Some("folder-9".into()),
        },
    );
    app.dispatch(
        &[0],
        Event::ListSelectionChanged {
            id: Some("folder-2".into()),
        },
    );

    assert_eq!(*calls.activated.borrow(), [FolderId(2)]);
    assert_eq!(*calls.deleted.borrow(), [FolderId(1)]);
    assert_eq!(*calls.checked.borrow(), [Some(FolderId(2))]);
    assert_eq!(app.get(selection), Some(FolderId(2)));

    // A rejected change leaves the Store on the current row.
    app.set(allow, false);
    app.dispatch(&[0], Event::ListSelectionChanged { id: None });
    assert_eq!(*calls.checked.borrow(), [Some(FolderId(2)), None]);
    assert_eq!(app.get(selection), Some(FolderId(2)));
}

#[test]
fn table_shows_the_bound_key_as_selected() {
    // Verifies a table shows the bound key's row as selected, and nothing for a key no row has.
    let (mut app, (selection, _)) = WithStores::mount(
        |cx| (cx.store(Some(FolderId(2))), cx.store(true)),
        |(selection, allow)| TableCalls::default().table(selection, allow),
    );
    let selected = |app: &Harness| match app.tree().value {
        RenderNode::View {
            kind: ViewKind::Table { rows, selected, .. },
            ..
        } => (rows, selected),
        other => panic!("expected a table, got {other:?}"),
    };
    assert_eq!(
        selected(&app),
        (
            vec!["folder-1".to_owned(), "folder-2".to_owned()],
            Some("folder-2".to_owned())
        )
    );

    app.set(selection, Some(FolderId(9)));

    assert_eq!(selected(&app).1, None);
}

#[test]
fn sidebar_selects_items_by_their_key() {
    // Verifies a sidebar shows its bound key, ignores unknown IDs, and reports chosen items as keys.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = seen.clone();
    let (mut app, selection) = WithStores::mount(
        |cx| cx.store(Some(FolderId(9))),
        move |selection| {
            let record = record.clone();
            Window::new("Folders")
                .size(Size::new(400.0, 300.0))
                .sidebar(
                    Sidebar::new([SidebarSection::new([
                        SidebarItem::new(FolderId(1), "Ideas"),
                        SidebarItem::new(FolderId(2), "Plans"),
                    ])])
                    .bind(selection)
                    .on_select(move |_, key| record.borrow_mut().push(*key)),
                )
        },
    );
    let selected = |app: &Harness| match app.tree().value {
        RenderNode::Window {
            sidebar: Some(sidebar),
            ..
        } => sidebar.selected_id,
        other => panic!("expected a window with a sidebar, got {other:?}"),
    };
    // No item has the bound key.
    assert_eq!(selected(&app), None);

    app.dispatch(
        &[0],
        Event::SidebarSelectionChanged {
            id: "folder-9".into(),
        },
    );
    app.dispatch(
        &[0],
        Event::SidebarSelectionChanged {
            id: "folder-2".into(),
        },
    );

    assert_eq!(app.get(selection), Some(FolderId(2)));
    assert_eq!(*seen.borrow(), [FolderId(2)]);
    assert_eq!(selected(&app).as_deref(), Some("folder-2"));
}

/// A row key from another crate, which cannot implement `ItemKey` here.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TransactionId(i64);

/// Builds a table of transactions 1 and 2 named by `item_id(..)`.
fn transaction_table(
    selection: Store<Option<TransactionId>>,
    seen: Rc<RefCell<Vec<Option<TransactionId>>>>,
) -> Table<TransactionId, ItemIdFn<TransactionId>> {
    Table::new(
        [TableColumn::new("Payee")],
        [1, 2].map(|number| {
            TableRow::new(
                TransactionId(number),
                list![Text::new(format!("Payee {number}"))],
            )
        }),
    )
    .item_id(|id: &TransactionId| format!("tx-{}", id.0))
    .bind(selection)
    .on_select(move |_, id| seen.borrow_mut().push(id.copied()))
}

#[test]
fn table_selects_foreign_keys_through_item_id() {
    // Verifies `item_id(..)` names rows of a non-`ItemKey` type and selection round-trips as that type.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = seen.clone();
    let (mut app, selection) = WithStores::mount(
        |cx| cx.store(Some(TransactionId(2))),
        move |selection| transaction_table(selection, record.clone()),
    );
    let rows = |app: &Harness| match app.tree().value {
        RenderNode::View {
            kind: ViewKind::Table { rows, selected, .. },
            ..
        } => (rows, selected),
        other => panic!("expected a table, got {other:?}"),
    };
    assert_eq!(
        rows(&app),
        (
            vec!["tx-1".to_owned(), "tx-2".to_owned()],
            Some("tx-2".to_owned())
        )
    );

    app.dispatch(
        &[0],
        Event::ListSelectionChanged {
            id: Some("tx-1".into()),
        },
    );

    assert_eq!(app.get(selection), Some(TransactionId(1)));
    assert_eq!(*seen.borrow(), [Some(TransactionId(1))]);
    assert_eq!(rows(&app).1.as_deref(), Some("tx-1"));
}

#[test]
#[should_panic(expected = "distinct item IDs")]
#[cfg(debug_assertions)]
fn table_catches_colliding_item_ids_in_debug_builds() {
    // Verifies distinct keys that `item_id(..)` gives the same string trip the debug assertion.
    Harness::new(
        Table::new(
            [TableColumn::new("Payee")],
            [1, 2].map(|number| TableRow::new(TransactionId(number), list![Text::new("Payee")])),
        )
        .item_id(|_: &TransactionId| "tx".to_owned()),
    );
}

#[test]
fn list_selects_foreign_keys_through_item_id() {
    // Verifies `item_id(..)` may follow other modifiers and still names a list's rows and selection.
    let (mut app, selection) = WithStores::mount(
        |cx| cx.store(Some(TransactionId(1))),
        |selection| {
            List::new([ListSection::new([1, 2].map(|number| {
                ListRow::new(TransactionId(number), Text::new(format!("Payee {number}")))
            }))])
            .bind(selection)
            .item_id(|id| format!("tx-{}", id.0))
        },
    );
    assert_eq!(
        list_rows(&app.tree()),
        (
            vec!["tx-1".to_owned(), "tx-2".to_owned()],
            Some("tx-1".to_owned())
        )
    );

    app.dispatch(
        &[0],
        Event::ListSelectionChanged {
            id: Some("tx-2".into()),
        },
    );

    assert_eq!(app.get(selection), Some(TransactionId(2)));
}

#[test]
fn sidebar_names_foreign_keys_through_item_id() {
    // Verifies a sidebar's item IDs, which are also accessibility identifiers, come from `item_id(..)`.
    let (mut app, selection) = WithStores::mount(
        |cx| cx.store(None::<TransactionId>),
        |selection| {
            Window::new("Accounts")
                .size(Size::new(400.0, 300.0))
                .sidebar(
                    Sidebar::new([SidebarSection::new([
                        SidebarItem::new(TransactionId(1), "Checking"),
                        SidebarItem::new(TransactionId(2), "Savings"),
                    ])])
                    .item_id(|id| format!("account-{}", id.0))
                    .bind(selection),
                )
        },
    );
    let sidebar = |app: &Harness| match app.tree().value {
        RenderNode::Window {
            sidebar: Some(sidebar),
            ..
        } => sidebar,
        other => panic!("expected a window with a sidebar, got {other:?}"),
    };
    let ids = sidebar(&app).sections[0]
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["account-1", "account-2"]);

    app.dispatch(
        &[0],
        Event::SidebarSelectionChanged {
            id: "account-2".into(),
        },
    );

    assert_eq!(app.get(selection), Some(TransactionId(2)));
    assert_eq!(sidebar(&app).selected_id.as_deref(), Some("account-2"));
}

/// Returns the accessibility description of a rendered view.
fn accessibility_of(node: &RenderedNode) -> &zintl_ui_backend_core::Accessibility {
    match &node.value {
        RenderNode::View { accessibility, .. } => accessibility,
        other => panic!("expected a view, got {other:?}"),
    }
}

#[test]
fn accessibility_modifiers_reach_any_view() {
    // Verifies every accessibility modifier lands on controls and containers alike.
    let app = Harness::new(
        VStack::new(list![
            Button::icon("plus", "Add train").accessibility_hint("Adds a train to the timetable"),
            Text::new("Speed")
                .id("speed-label")
                .accessibility_hidden(true),
            Slider::new(0.0..=100.0)
                .accessibility_labeled_by("speed-label")
                .accessibility_value("Fast"),
        ])
        .accessibility_label("Controls")
        .accessibility_role(AccessibilityRole::Group),
    );
    let tree = app.tree();

    let stack = accessibility_of(&tree);
    assert_eq!(stack.label.as_deref(), Some("Controls"));
    assert_eq!(stack.role, Some(AccessibilityRole::Group));
    let button = accessibility_of(&tree.children[0]);
    assert_eq!(button.label.as_deref(), Some("Add train"));
    assert_eq!(
        button.hint.as_deref(),
        Some("Adds a train to the timetable")
    );
    assert!(accessibility_of(&tree.children[1]).hidden);
    let slider = accessibility_of(&tree.children[2]);
    assert_eq!(slider.labeled_by.as_deref(), Some("speed-label"));
    assert_eq!(slider.value.as_deref(), Some("Fast"));
}

#[test]
fn views_without_modifiers_keep_the_platform_description() {
    // Verifies unmodified views carry no accessibility overrides.
    let app = Harness::new(Button::new("Save"));

    assert!(accessibility_of(&app.tree()).is_default());
}

#[test]
fn disclosure_group_names_its_section_and_triangle() {
    // Verifies the group and its triangle carry the title, and the visible title is not read twice.
    let app = Harness::new(DisclosureGroup::new("Line settings", Text::new("Details")));
    let tree = app.tree();
    let header = &tree.children[0];

    let group = accessibility_of(&tree);
    assert_eq!(group.label.as_deref(), Some("Line settings"));
    assert_eq!(group.role, Some(AccessibilityRole::Group));
    assert_eq!(
        accessibility_of(&header.children[0]).label.as_deref(),
        Some("Line settings")
    );
    assert!(accessibility_of(&header.children[1]).hidden);
}

#[test]
fn modifiers_override_a_views_own_description() {
    // Verifies a modifier on a disclosure group replaces its default label but keeps its role.
    let app = Harness::new(
        DisclosureGroup::new("Line settings", Text::new("Details")).accessibility_label("Lines"),
    );
    let group = accessibility_of(&app.tree()).clone();

    assert_eq!(group.label.as_deref(), Some("Lines"));
    assert_eq!(group.role, Some(AccessibilityRole::Group));
}

#[test]
fn actions_queue_announcements() {
    // Verifies an action's announcement waits in the queue the backend posts from.
    zintl_ui_backend_core::Announcements::take();
    let mut app = Harness::new(Button::new("Save").on_click(|cx| {
        cx.announce("Saved", AnnouncementPriority::High);
    }));

    app.dispatch(&[0], Event::Activated);

    let announcements = zintl_ui_backend_core::Announcements::take();
    assert_eq!(announcements.len(), 1);
    assert_eq!(announcements[0].text, "Saved");
    assert_eq!(announcements[0].priority, AnnouncementPriority::High);
}

/// Returns the first node of `kind` below `node`, depth first.
fn find_kind(node: &RenderedNode, kind: fn(&ViewKind) -> bool) -> Option<ViewKind> {
    match &node.value {
        RenderNode::View { kind: found, .. } if kind(found) => Some(found.clone()),
        _ => node
            .children
            .iter()
            .find_map(|child| find_kind(child, kind)),
    }
}

#[test]
fn controls_show_value_and_prefer_a_bound_store() {
    // Verifies each control shows `value` alone, its Store alone, and the Store when given both.
    let red = Color::rgb(1.0, 0.0, 0.0);
    let (app, _) = WithStores::mount(
        |cx| {
            (
                cx.store(true),
                cx.store(0.75),
                cx.store("bound".to_owned()),
                cx.store(Color::BLACK),
                cx.store(2_usize),
            )
        },
        move |(on, level, text, color, index)| {
            VStack::new(list![
                // `value` alone.
                Toggle::new("").value(true),
                Slider::new(0.0..=1.0).value(0.25),
                SearchField::new().value("typed"),
                ComboBox::new(["a"]).value("typed"),
                ColorWell::new().value(red),
                Picker::indexed(["a", "b", "c"]).selection(1),
                // The Store wins, whichever is declared first.
                Toggle::new("").value(false).bind(on),
                Slider::new(0.0..=1.0).bind(level).value(0.25),
                SearchField::new().value("typed").bind(text),
                ComboBox::new(["a"]).bind(text).value("typed"),
                ColorWell::new().value(red).bind(color),
                Picker::indexed(["a", "b", "c"]).bind(index).selection(1),
                // The Store alone.
                Toggle::new("").bind(on),
                Slider::new(0.0..=1.0).bind(level),
                SearchField::new().bind(text),
                ComboBox::new(["a"]).bind(text),
                ColorWell::new().bind(color),
                Picker::indexed(["a", "b", "c"]).bind(index),
            ])
        },
    );
    let kinds: Vec<_> = app
        .tree()
        .children
        .iter()
        .map(|child| match &child.value {
            RenderNode::View { kind, .. } => kind.clone(),
            _ => panic!("expected a view"),
        })
        .collect();
    let shown = |kind: &ViewKind| match kind {
        ViewKind::Switch { on } => format!("{on}"),
        ViewKind::Slider { value, .. } => format!("{value}"),
        ViewKind::SearchField { value, .. } | ViewKind::ComboBox { value, .. } => value.clone(),
        ViewKind::ColorWell { color } => format!("{}", *color == red),
        ViewKind::PopUpButton { selected, .. } => format!("{selected:?}"),
        _ => panic!("unexpected control"),
    };
    let shown: Vec<_> = kinds.iter().map(shown).collect();
    assert_eq!(
        shown[..6],
        ["true", "0.25", "typed", "typed", "true", "Some(1)"]
    );
    let bound = ["true", "0.75", "bound", "bound", "false", "Some(2)"];
    assert_eq!(shown[6..12], bound);
    assert_eq!(shown[12..], bound);
}

#[test]
fn discrete_controls_keep_their_value_when_on_change_ignores_the_change() {
    // Verifies a toggle, picker, and tab view with a fixed value still render it after a change nobody stores.
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    let mut app = Harness::new(VStack::new(list![
        Toggle::new("").value(true).on_change({
            let count = count.clone();
            move |_, _| count.set(count.get() + 1)
        }),
        Picker::indexed(["a", "b"]).selection(1).on_change({
            let count = count.clone();
            move |_, _| count.set(count.get() + 1)
        }),
        TabView::new([
            Tab::new(0, "One", Text::new("1")),
            Tab::new(1, "Two", Text::new("2"))
        ])
        .selection(1)
        .on_change(move |_, _| count.set(count.get() + 1)),
    ]));
    assert!(app.dispatch(&[0, 0], Event::StateChanged { on: false }));
    assert!(app.dispatch(&[0, 1], Event::SelectionChanged { index: 0 }));
    assert!(app.dispatch(&[0, 2], Event::SelectionChanged { index: 0 }));

    assert_eq!(changes.get(), 3);
    assert!(
        find_kind(&app.tree(), |kind| matches!(
            kind,
            ViewKind::Switch { on: true }
        ))
        .is_some()
    );
    assert!(
        find_kind(&app.tree(), |kind| matches!(
            kind,
            ViewKind::PopUpButton {
                selected: Some(1),
                ..
            }
        ))
        .is_some()
    );
    assert!(
        find_kind(&app.tree(), |kind| matches!(
            kind,
            ViewKind::TabView { selected: 1, .. }
        ))
        .is_some()
    );
}

#[test]
fn text_controls_report_edits_as_str() {
    // Verifies combo boxes and search fields pass the edited text to `on_change` as `&str`.
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = |seen: &Rc<RefCell<Vec<String>>>| {
        let seen = seen.clone();
        move |_: &mut Context<'_>, text: &str| seen.borrow_mut().push(text.to_owned())
    };
    let mut app = Harness::new(VStack::new(list![
        ComboBox::new(["a"]).on_change(record(&seen)),
        SearchField::new().on_change(record(&seen)),
    ]));

    app.dispatch(&[0, 0], Event::TextChanged { value: "c".into() });
    app.dispatch(&[0, 1], Event::TextChanged { value: "s".into() });

    assert_eq!(*seen.borrow(), ["c", "s"]);
}

#[test]
fn focus_trigger_counts_fires_and_starts_at_zero() {
    // Verifies a field mounted with an unfired trigger requests no focus, and each fire adds one request.
    let (mut app, focus) = WithStores::mount(
        |cx| cx.trigger(),
        |focus| VStack::new(list![TextField::new().focus_on(focus)]),
    );
    let requests = |app: &Harness| match find_kind(&app.tree(), |kind| {
        matches!(kind, ViewKind::TextField { .. })
    }) {
        Some(ViewKind::TextField { focus, .. }) => focus,
        _ => panic!("expected a text field"),
    };
    assert_eq!(requests(&app), 0);

    app.fire(focus, ());
    assert_eq!(requests(&app), 1);
    // An equal value fires again.
    app.fire(focus, ());
    assert_eq!(requests(&app), 2);
}

#[test]
fn slider_ticks_and_snapping_are_independent() {
    // Verifies tick marks and snapping are separate settings whose call order does not matter.
    let slider = |slider: Slider| match Harness::new(slider).tree().value {
        RenderNode::View {
            kind:
                ViewKind::Slider {
                    tick_marks,
                    snaps_to_ticks,
                    ..
                },
            ..
        } => (tick_marks, snaps_to_ticks),
        other => panic!("expected a slider, got {other:?}"),
    };

    assert_eq!(
        slider(Slider::new(0.0..=1.0).ticks(5).snaps_to_ticks(true)),
        (5, true)
    );
    assert_eq!(
        slider(Slider::new(0.0..=1.0).snaps_to_ticks(true).ticks(5)),
        (5, true)
    );
    assert_eq!(slider(Slider::new(0.0..=1.0).ticks(5)), (5, false));
}

#[test]
fn icon_and_help_buttons_have_no_title_and_measure_their_size() {
    // Verifies icon buttons carry their label for accessibility, which a later label replaces, and help buttons use the help bezel.
    let button = |button: Button| match Harness::new(button).tree().value {
        RenderNode::View {
            kind: ViewKind::Button(button),
            layout,
            accessibility,
            ..
        } => (button, layout.minimum_size, accessibility.label),
        other => panic!("expected a button, got {other:?}"),
    };
    let (icon, icon_minimum, icon_label) = button(Button::icon("gearshape", "Settings"));
    let (_, _, replaced) =
        button(Button::icon("gearshape", "Settings").accessibility_label("Preferences"));
    let (help, help_minimum, _) = button(Button::help());

    assert_eq!(icon.title, "");
    assert_eq!(icon.system_image.as_deref(), Some("gearshape"));
    assert_eq!(icon_minimum, Some(Size::default()));
    assert_eq!(icon_label.as_deref(), Some("Settings"));
    assert_eq!(replaced.as_deref(), Some("Preferences"));
    assert_eq!(help.title, "");
    assert_eq!(help.bezel, Some(BezelStyle::Help));
    assert_eq!(help_minimum, Some(Size::default()));
}

#[test]
fn table_column_width_is_a_layout_length() {
    // Verifies a column width reaches the node as an f32 layout length.
    let node = Harness::new(Table::new(
        [TableColumn::new("Name").width(120.0)],
        Vec::<TableRow>::new(),
    ))
    .tree()
    .value;
    let RenderNode::View {
        kind: ViewKind::Table { columns, .. },
        ..
    } = node
    else {
        panic!("expected a table, got {node:?}");
    };

    assert_eq!(columns[0].width, Some(120.0_f32));
}

#[test]
fn menu_picker_reserves_more_width_for_wide_characters() {
    // Verifies a pop-up picker estimates full-width items wider than ASCII items of the same length.
    let minimum = |picker: Picker<usize>| match Harness::new(picker).tree().value {
        RenderNode::View { layout, .. } => layout.minimum_size.map_or(0.0, |size| size.width),
        other => panic!("expected a pop-up button, got {other:?}"),
    };

    assert!(minimum(Picker::indexed(["設定項目名前"])) > minimum(Picker::indexed(["abcdef"])));
}

#[test]
fn palette_resolves_separator_and_alternating_row() {
    // Verifies the new palette colors resolve to their fields, and the light defaults keep the colors drawn before.
    let palette = Palette {
        separator: Color::rgb(1.0, 0.0, 0.0),
        alternating_row: Color::rgb(0.0, 1.0, 0.0),
        ..Palette::LIGHT
    };

    assert_eq!(palette.resolve(PaletteColor::Separator), palette.separator);
    assert_eq!(
        palette.resolve(PaletteColor::AlternatingRow),
        palette.alternating_row
    );
    assert_eq!(Palette::LIGHT.separator, Color::rgba(0.5, 0.5, 0.5, 0.35));
    assert_eq!(
        Palette::LIGHT.alternating_row,
        Color::rgba(0.5, 0.5, 0.5, 0.1)
    );
}
