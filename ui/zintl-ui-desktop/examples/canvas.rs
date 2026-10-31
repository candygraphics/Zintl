//! A sketch pad drawn with `Canvas2D`.
//!
//! Drag to draw, type to add a caption, and use the arrow keys to move the
//! marker after clicking the canvas. "Animate" switches between on-demand and
//! continuous redraw.

use std::f64::consts::TAU;

use zintl_ui_desktop::*;

/// Declarations the canvas receives on each render.
#[derive(Clone, Copy, Default, PartialEq)]
struct Settings {
    animate: bool,
    /// Incremented by "Clear"; a change erases the strokes.
    clears: u32,
}

/// Drawing state owned by the canvas; it survives view updates.
#[derive(Clone)]
struct Sketch {
    settings: Settings,
    strokes: Vec<Vec<Point>>,
    pointer: Option<Point>,
    marker: Point,
    caption: String,
    focused: bool,
    font: Font,
}

impl Sketch {
    fn new(settings: Settings) -> Self {
        Self {
            settings,
            strokes: Vec::new(),
            pointer: None,
            marker: Point::new(40.0, 40.0),
            caption: String::new(),
            focused: false,
            font: Font::system(15.0),
        }
    }

    fn key(&mut self, key: &KeyEvent, cx: &mut CanvasEventContext) -> bool {
        if key.phase != KeyPhase::Down {
            return false;
        }
        let step = if key.modifiers.shift { 20.0 } else { 4.0 };
        match &key.key {
            Key::Named(NamedKey::ArrowLeft) => self.marker.x -= step,
            Key::Named(NamedKey::ArrowRight) => self.marker.x += step,
            Key::Named(NamedKey::ArrowUp) => self.marker.y -= step,
            Key::Named(NamedKey::ArrowDown) => self.marker.y += step,
            Key::Named(NamedKey::Backspace) => {
                self.caption.pop();
            }
            Key::Named(NamedKey::Escape) => cx.resign_focus(),
            // Leave shortcuts such as Command-Q to the menu.
            _ if key.modifiers.meta || key.modifiers.control => return false,
            _ => match &key.text {
                Some(text) => self.caption.push_str(text),
                None => return false,
            },
        }
        cx.invalidate();
        true
    }
}

impl Canvas2DHandler for Sketch {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        let bounds = frame.bounds;
        g.clear(Color::rgb(0.98, 0.98, 0.96));

        let background = Paint::linear(
            Point::new(0.0, 0.0),
            Point::new(0.0, bounds.height),
            [
                GradientStop::new(0.0, Color::rgba(0.3, 0.5, 0.9, 0.08)),
                GradientStop::new(1.0, Color::rgba(0.3, 0.5, 0.9, 0.0)),
            ],
        );
        g.fill_rect(bounds, background);

        let ink = StrokeStyle::new(3.0)
            .cap(LineCap::Round)
            .join(LineJoin::Round);
        for stroke in &self.strokes {
            let mut path = Path::new();
            for point in stroke {
                path.line_to(*point);
            }
            g.stroke_path(&path, Color::rgb(0.15, 0.2, 0.35), &ink);
        }

        if self.settings.animate {
            // Orbit the marker once every three seconds.
            let angle = frame.time / 3.0 * TAU;
            let orbit = Point::new(
                self.marker.x + 24.0 * angle.cos(),
                self.marker.y + 24.0 * angle.sin(),
            );
            g.fill_path(&Path::circle(orbit, 5.0), Color::rgb(0.95, 0.55, 0.2));
        }
        g.fill_path(
            &Path::rounded_rect(
                Rect::new(self.marker.x - 8.0, self.marker.y - 8.0, 16.0, 16.0),
                4.0,
            ),
            Color::rgb(0.2, 0.6, 0.4),
        );

        if let Some(pointer) = self.pointer {
            g.stroke_path(
                &Path::circle(pointer, 10.0),
                Color::rgba(0.0, 0.0, 0.0, 0.3),
                &StrokeStyle::new(1.0).dashes([3.0, 3.0], 0.0),
            );
        }

        let caption = if self.caption.is_empty() {
            "Click, then type a caption"
        } else {
            &self.caption
        };
        let metrics = g.measure_text(caption, &self.font);
        g.fill_text(
            caption,
            Point::new(
                (bounds.width - metrics.width) / 2.0,
                bounds.height - 16.0 - metrics.descent,
            ),
            &self.font,
            Color::rgb(0.25, 0.25, 0.25),
        );

        if self.focused {
            g.stroke_rect(
                bounds.inset(1.5),
                Color::rgba(0.2, 0.45, 0.95, 0.8),
                &StrokeStyle::new(3.0),
            );
        }
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        match event {
            CanvasEvent::Pointer(pointer) => {
                match (pointer.phase, pointer.button) {
                    (PointerPhase::Down, Some(PointerButton::Primary)) => {
                        self.strokes.push(vec![pointer.location]);
                    }
                    (PointerPhase::Moved, Some(PointerButton::Primary)) => {
                        if let Some(stroke) = self.strokes.last_mut() {
                            stroke.push(pointer.location);
                        }
                    }
                    (PointerPhase::Exited, _) => self.pointer = None,
                    _ => {}
                }
                if pointer.phase != PointerPhase::Exited {
                    self.pointer = Some(pointer.location);
                }
                cx.invalidate();
                true
            }
            CanvasEvent::Key(key) => self.key(key, cx),
            CanvasEvent::Focus(focused) => {
                self.focused = *focused;
                cx.invalidate();
                true
            }
            _ => false,
        }
    }

    fn reconcile(&mut self, next: Self) -> Redraw {
        if next.settings == self.settings {
            return Redraw::No;
        }
        if next.settings.clears != self.settings.clears {
            self.strokes.clear();
            self.caption.clear();
        }
        self.settings = next.settings;
        Redraw::Yes
    }
}

#[derive(Default)]
struct CanvasExample {
    settings: Store<Settings>,
    strokes: Store<usize>,
}

impl View for CanvasExample {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        self.settings = cx.store(Settings::default());
        self.strokes = cx.store(0);
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let settings = self.settings;
        let strokes = self.strokes;
        Window::new(Rect::new(120.0, 120.0, 720.0, 520.0), "Canvas").content(
            VStack::new(list![
                cx.watch(settings, move |current| {
                    HStack::new(list![
                        Toggle::new("Animate")
                            .on(current.animate)
                            .on_change(move |cx, on| {
                                cx.update(settings, |settings| settings.animate = on)
                            }),
                        Button::new("Clear").on_click(move |cx| {
                            cx.update(settings, |settings| settings.clears += 1);
                            cx.update(strokes, |count| *count = 0);
                        }),
                    ])
                    .spacing(12.0)
                }),
                cx.watch(strokes, |count| Text::new(format!("Strokes: {count}"))),
                cx.watch(settings, move |current| {
                    let redraw = if current.animate {
                        RedrawPolicy::Continuous {
                            preferred_fps: None,
                        }
                    } else {
                        RedrawPolicy::OnDemand
                    };
                    Canvas2D::new(Sketch::new(*current))
                        .redraw(redraw)
                        .focusable(true)
                        .fill_width()
                        .grow()
                        .on_event(move |cx, event| {
                            // Store updates go through the view cycle, so only count finished strokes.
                            if let CanvasEvent::Pointer(PointerEvent {
                                phase: PointerPhase::Up,
                                button: Some(PointerButton::Primary),
                                ..
                            }) = event
                            {
                                cx.update(strokes, |count| *count += 1);
                            }
                        })
                }),
            ])
            .spacing(12.0)
            .padding(16.0)
            .fill_width()
            .grow(),
        )
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> Result<(), AppError> {
    App::new(CanvasExample::default()).run()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
