use zintl_ui_desktop::*;

/// What the pad reports to its view.
enum PadAction {
    Hover(bool),
    /// A click with the number of clicks the mounted handler has counted.
    Clicked(u32),
    Key(String),
    Scrolled,
}

/// Counts clicks in handler state, which reconcile keeps across renders.
#[derive(Clone)]
struct Pad {
    clicks: u32,
    tint: f64,
}

impl Canvas2DHandler for Pad {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        g.fill_rect(frame.bounds, Color::rgba(0.2, self.tint, 0.8, 0.3));
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        match event {
            CanvasEvent::Pointer(pointer) => match pointer.phase {
                PointerPhase::Entered => cx.emit(PadAction::Hover(true)),
                PointerPhase::Exited => cx.emit(PadAction::Hover(false)),
                PointerPhase::Down => {
                    self.clicks += 1;
                    cx.request_focus();
                    cx.emit(PadAction::Clicked(self.clicks));
                }
                _ => {}
            },
            CanvasEvent::Key(key) if key.phase == KeyPhase::Down => {
                let Key::Character(character) = &key.key else {
                    return false;
                };
                cx.emit(PadAction::Key(character.clone()));
            }
            CanvasEvent::Scroll(_) => cx.emit(PadAction::Scrolled),
            _ => return false,
        }
        true
    }

    // Takes the new tint but keeps the click count.
    fn reconcile(&mut self, next: Self) -> Redraw {
        self.tint = next.tint;
        Redraw::Yes
    }
}

/// A focusable canvas reporting hover, clicks, keys, and scrolling.
pub struct MainView {
    hover: Store<bool>,
    clicks: Store<u32>,
    key: Store<String>,
    scrolls: Store<u32>,
    renders: Store<u32>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            hover: Store::default(),
            clicks: Store::default(),
            key: Store::default(),
            scrolls: Store::default(),
            renders: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.hover = cx.store(false);
        self.clicks = cx.store(0);
        self.key = cx.store(String::new());
        self.scrolls = cx.store(0);
        self.renders = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let (hover, clicks, key, scrolls, renders) =
            (self.hover, self.clicks, self.key, self.scrolls, self.renders);
        let tint = if *cx.get(renders) % 2 == 0 { 0.4 } else { 0.7 };
        Window::new(Rect::new(100.0, 100.0, 480.0, 400.0), "Zintl Canvas Input UI Test")
            .id("main-window")
            .content(
                VStack::new(list![
                    cx.watch(hover, |hover| Text::new(format!("Hover: {hover}")).id("hover")),
                    cx.watch(clicks, |clicks| Text::new(format!("Clicks: {clicks}")).id("clicks")),
                    cx.watch(key, |key| Text::new(format!("Key: {key}")).id("key")),
                    cx.watch(scrolls, |scrolls| Text::new(format!("Scrolls: {scrolls}"))
                        .id("scrolls")),
                    Button::new("Re-render")
                        .id("rerender")
                        .on_click(move |cx| cx.update(renders, |renders| *renders += 1)),
                    Canvas2D::new(Pad { clicks: 0, tint })
                        .focusable(true)
                        .on_action(move |cx, action: &PadAction| match action {
                            PadAction::Hover(inside) => {
                                let inside = *inside;
                                cx.update(hover, |hover| *hover = inside);
                            }
                            PadAction::Clicked(count) => {
                                let count = *count;
                                cx.update(clicks, |clicks| *clicks = count);
                            }
                            PadAction::Key(character) => {
                                let character = character.clone();
                                cx.update(key, |key| *key = character);
                            }
                            PadAction::Scrolled => cx.update(scrolls, |scrolls| *scrolls += 1),
                        })
                        .fill_width()
                        .grow(),
                ])
                .spacing(8.0)
                .fill_width()
                .grow(),
            )
    }
}
