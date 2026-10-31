//! A `Canvas2D` that fills its window under a hidden title bar.
//!
//! On macOS only the window buttons stay above the drawing. Move the pointer
//! to pull the particles; click to toggle the animation. Clicks in the title
//! bar area move the window instead of reaching the canvas; mark a view with
//! `.title_bar_interactive()` to receive them there.

use std::f64::consts::TAU;

use zintl_ui_desktop::*;

const PARTICLES: usize = 48;

/// Particles orbiting the pointer, animated every display refresh.
#[derive(Clone)]
struct Orbits {
    /// The point the particles circle, in canvas points.
    center: Option<Point>,
    /// Animation time that stops advancing while paused.
    elapsed: f64,
    paused: bool,
    font: Font,
}

impl Orbits {
    fn new() -> Self {
        Self {
            center: None,
            elapsed: 0.0,
            paused: false,
            font: Font::system(13.0).weight(FontWeight::MEDIUM),
        }
    }
}

impl Canvas2DHandler for Orbits {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        if !self.paused {
            self.elapsed += frame.delta;
        }
        let bounds = frame.bounds;
        let center = self.center.unwrap_or_else(|| bounds.center());
        let t = self.elapsed;

        g.fill_rect(
            bounds,
            Paint::linear(
                Point::new(0.0, 0.0),
                Point::new(bounds.width, bounds.height),
                [
                    GradientStop::new(0.0, Color::rgb(0.06, 0.07, 0.16)),
                    GradientStop::new(1.0, Color::rgb(0.16, 0.05, 0.22)),
                ],
            ),
        );
        g.fill_rect(
            bounds,
            Paint::radial(
                center,
                bounds.width.max(bounds.height) * 0.4,
                [
                    GradientStop::new(0.0, Color::rgba(0.45, 0.35, 1.0, 0.35)),
                    GradientStop::new(1.0, Color::rgba(0.45, 0.35, 1.0, 0.0)),
                ],
            ),
        );

        let reach = bounds.width.min(bounds.height) * 0.42;
        for index in 0..PARTICLES {
            let share = index as f64 / PARTICLES as f64;
            let radius = reach * (0.25 + 0.75 * share);
            let speed = 0.3 + 0.9 * (1.0 - share);
            let angle = share * TAU * 7.0 + t * speed;
            let point = Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin() * 0.6,
            );
            let hue = (share + t * 0.05).fract();
            let color = Color::rgba(
                0.6 + 0.4 * (hue * TAU).cos(),
                0.5 + 0.4 * ((hue + 0.33) * TAU).cos(),
                0.8 + 0.2 * ((hue + 0.66) * TAU).cos(),
                0.9,
            );
            g.fill_path(&Path::circle(point, 2.0 + 4.0 * (1.0 - share)), color);
        }

        let hint = if self.paused {
            "Paused — click to resume"
        } else {
            "Move the pointer · click to pause"
        };
        let metrics = g.measure_text(hint, &self.font);
        g.fill_text(
            hint,
            Point::new((bounds.width - metrics.width) / 2.0, bounds.height - 24.0),
            &self.font,
            Color::rgba(1.0, 1.0, 1.0, 0.6),
        );
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        let CanvasEvent::Pointer(pointer) = event else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Moved | PointerPhase::Entered => self.center = Some(pointer.location),
            PointerPhase::Exited => self.center = None,
            PointerPhase::Down => self.paused = !self.paused,
            PointerPhase::Up => {}
        }
        cx.invalidate();
        true
    }

    // The canvas has no declared inputs, so later renders keep the animation state.
    fn reconcile(&mut self, _next: Self) -> Redraw {
        Redraw::No
    }
}

struct FullscreenCanvas;

impl View for FullscreenCanvas {
    type Output = RenderNode;

    fn render(&self, _cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        Window::new(Rect::new(120.0, 120.0, 900.0, 600.0), "Orbits")
            .title_bar(TitleBar::Hidden)
            .content(
                Canvas2D::new(Orbits::new())
                    .redraw(RedrawPolicy::Continuous {
                        preferred_fps: None,
                    })
                    .fill_width()
                    .grow(),
            )
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> Result<(), AppError> {
    App::new(FullscreenCanvas).run()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
