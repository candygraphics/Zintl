use zintl_ui_desktop::*;

/// A text-field date picker and a calendar bound to one date, with a button that sets a leap day.
pub struct MainView {
    date: Store<CalendarDate>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            date: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.date = cx.store(CalendarDate {
            year: 2026,
            month: 11,
            day: 15,
        });
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let date = self.date;
        Window::new(Rect::new(100.0, 100.0, 420.0, 360.0), "Zintl Date Picker UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    DatePicker::new().bind(date).id("date"),
                    DatePicker::new()
                        .style(DatePickerStyle::Graphical)
                        .bind(date)
                        .id("calendar"),
                    Button::new("Set leap day").id("set-leap").on_click(move |cx| {
                        cx.update(date, |date| {
                            *date = CalendarDate {
                                year: 2024,
                                month: 2,
                                day: 29,
                            }
                        })
                    }),
                    cx.watch(date, |date| Text::new(format!("Date: {date}")).id("date-value")),
                ])
                .alignment(CrossAxisAlignment::Start)
                .spacing(10.0),
            )
    }
}
