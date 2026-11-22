use zintl_ui_desktop::*;

/// A text-field date picker and a calendar bound to one date, with a button that sets a leap day,
/// and an optional date picker with a button that sets its date.
pub struct MainView {
    date: Store<CalendarDate>,
    due: Store<Option<CalendarDate>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            date: Store::default(),
            due: Store::default(),
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
        self.due = cx.store(None);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (date, due) = (self.date, self.due);
        Window::new("Zintl Date Picker UI Test").size(Size::new(420.0, 460.0))
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
                    DatePicker::optional()
                        .placeholder("No date")
                        .bind(due)
                        .id("due"),
                    Button::new("Set due date").id("set-due").on_click(move |cx| {
                        cx.update(due, |due| *due = CalendarDate::new(2026, 12, 24))
                    }),
                    cx.watch(due, |due| {
                        let due = due.map_or("No date".to_owned(), |due| due.to_string());
                        Text::new(format!("Due: {due}")).id("due-value")
                    }),
                ])
                .alignment(CrossAxisAlignment::Start)
                .spacing(10.0),
            )
    }
}
