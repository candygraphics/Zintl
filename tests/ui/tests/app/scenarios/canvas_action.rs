use zintl_ui_desktop::*;

/// What the strip reports to its view.
enum StripAction {
    Tapped {
        clicks: u32,
    },
    /// A drag ended after moving `distance` points to the right.
    Dragged {
        distance: f64,
    },
}

/// Emits taps and finished drags; the drag state stays in the handler.
#[derive(Clone)]
struct Strip {
    grab: Option<f64>,
}

impl Canvas2DHandler for Strip {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        g.fill_rect(frame.bounds, Color::rgba(0.2, 0.4, 0.8, 0.3));
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        let CanvasEvent::Pointer(pointer) = event else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Entered => cx.set_cursor(Cursor::PointingHand),
            PointerPhase::Down => {
                self.grab = Some(pointer.location.x);
                cx.emit(StripAction::Tapped {
                    clicks: pointer.click_count,
                });
            }
            PointerPhase::Up => {
                if let Some(grab) = self.grab.take() {
                    let distance = pointer.location.x - grab;
                    if distance.abs() > 4.0 {
                        cx.emit(StripAction::Dragged { distance });
                    }
                }
            }
            _ => {}
        }
        true
    }

    fn reconcile(&mut self, _next: Self) -> Redraw {
        Redraw::No
    }
}

pub struct MainView {
    status: Store<String>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            status: Store::default(),
        }
    }
}

impl View for MainView {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.status = cx.store("Idle".to_owned());
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let status = self.status;
        Window::new(
            Rect::new(100.0, 100.0, 480.0, 320.0),
            "Zintl Canvas Action UI Test",
        )
        .id("main-window")
        .content(
            VStack::new(list![
                cx.watch(status, |status| Text::new(status.clone())
                    .id("canvas-status")),
                Canvas2D::new(Strip { grab: None })
                    .on_action(move |cx, action: &StripAction| {
                        let text = match action {
                            StripAction::Tapped { clicks } => format!("Tapped {clicks}"),
                            StripAction::Dragged { distance } => {
                                format!("Dragged {}", distance.round())
                            }
                        };
                        cx.update(status, |status| *status = text);
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
