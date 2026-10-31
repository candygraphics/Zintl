//! Connects core canvas handlers to `zpd-appkit` canvas views.

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::rc::Rc;
use std::sync::Arc;

use zpd_appkit::geometry;
use zpd_appkit::graphics::{self as native, GraphicsContext};
use zpd_appkit::ui::{CanvasDelegate, CanvasFrame, CanvasHandle, CanvasInput, CanvasView};

use crate::convert::{Native, Shared};
use crate::graphics::{
    Color, FillRule, Font, Graphics2D, GraphicsBackend, Image, Paint, Path, PathElement, Point,
    Rect, StrokeStyle, TextMetrics, Transform,
};
use crate::{CanvasHost, CanvasNode, Frame, MainTaskSender};

/// Native resources a canvas reuses across frames.
#[derive(Default)]
struct Resources {
    fonts: HashMap<Font, native::Font>,
    /// Images by [`Image::id`], with the frame that last drew them.
    images: HashMap<u64, (native::Image, u64)>,
    frame: u64,
}

impl Resources {
    fn font(&mut self, font: &Font) -> &native::Font {
        self.fonts
            .entry(font.clone())
            .or_insert_with(|| font.native())
    }

    /// Drops images the last frame did not draw.
    fn end_frame(&mut self) {
        let frame = self.frame;
        self.images.retain(|_, (_, used)| *used == frame);
        self.frame += 1;
    }
}

/// Draws and routes input for one mounted canvas.
pub(super) struct CanvasBridge {
    canvas: CanvasNode,
    resources: RefCell<Resources>,
}

impl CanvasBridge {
    /// Installs `canvas` on `view` and connects its host operations.
    pub(super) fn install(view: &CanvasView, canvas: &CanvasNode, sender: Arc<MainTaskSender>) {
        view.set_delegate(Rc::new(Self {
            canvas: canvas.clone(),
            resources: RefCell::default(),
        }));
        canvas.attach(Rc::new(Host(view.handle())), Some(sender));
    }
}

impl CanvasDelegate for CanvasBridge {
    fn draw(&self, context: &mut GraphicsContext<'_>, frame: CanvasFrame) {
        let mut resources = self.resources.borrow_mut();
        let bounds = frame.bounds.shared();
        let mut backend = CoreGraphics {
            context,
            resources: &mut resources,
        };
        let mut graphics = Graphics2D::new(&mut backend, bounds, frame.scale);
        self.canvas.draw(
            &mut graphics,
            &Frame {
                bounds,
                safe_area: frame.safe_area.shared(),
                scale: frame.scale,
                time: frame.time,
                delta: frame.delta,
            },
        );
        resources.end_frame();
    }

    fn input(&self, input: &CanvasInput) -> bool {
        self.canvas.event(&input.clone().shared())
    }

    fn title_bar_hit(&self, location: geometry::Point) -> bool {
        self.canvas
            .title_bar_hit(Point::new(location.x, location.y))
    }
}

/// Performs a canvas's native requests through its view.
struct Host(CanvasHandle);

impl CanvasHost for Host {
    fn invalidate(&self) {
        // A view that is already gone has nothing to draw.
        let _ = self.0.set_needs_display();
    }

    fn set_focus(&self, focused: bool) {
        let _ = if focused {
            self.0.focus().map(drop)
        } else {
            self.0.resign_focus()
        };
    }
}

/// [`GraphicsBackend`] over a Core Graphics context.
struct CoreGraphics<'a, 'frame> {
    context: &'a mut GraphicsContext<'frame>,
    resources: &'a mut Resources,
}

impl CoreGraphics<'_, '_> {
    fn add_path(&mut self, path: &Path) {
        self.context.begin_path();
        for element in path.elements() {
            match *element {
                PathElement::MoveTo(point) => self.context.move_to(point.native()),
                PathElement::LineTo(point) => self.context.line_to(point.native()),
                PathElement::QuadTo(control, point) => {
                    self.context.quad_to(control.native(), point.native())
                }
                PathElement::CubicTo(first, second, point) => {
                    self.context
                        .cubic_to(first.native(), second.native(), point.native())
                }
                PathElement::Close => self.context.close_path(),
            }
        }
    }

    /// Paints a gradient through the current path, which becomes the clip.
    fn paint_gradient(&mut self, paint: &Paint, rule: FillRule) {
        self.context.save();
        self.context.clip(rule.native());
        match paint {
            Paint::Solid(_) => {}
            Paint::Linear(gradient) => self.context.draw_linear_gradient(
                &gradient
                    .stops
                    .iter()
                    .map(Native::native)
                    .collect::<Vec<_>>(),
                gradient.start.native(),
                gradient.end.native(),
            ),
            Paint::Radial(gradient) => self.context.draw_radial_gradient(
                &gradient
                    .stops
                    .iter()
                    .map(Native::native)
                    .collect::<Vec<_>>(),
                gradient.center.native(),
                gradient.radius,
            ),
        }
        self.context.restore();
    }

    fn apply_stroke(&mut self, style: &StrokeStyle) {
        self.context.set_line_width(style.width);
        self.context.set_line_cap(style.cap.native());
        self.context.set_line_join(style.join.native());
        self.context.set_miter_limit(style.miter_limit);
        self.context.set_line_dash(style.dash_offset, &style.dashes);
    }
}

impl GraphicsBackend for CoreGraphics<'_, '_> {
    fn save(&mut self) {
        self.context.save();
    }

    fn restore(&mut self) {
        self.context.restore();
    }

    fn concat(&mut self, transform: Transform) {
        self.context.concat(transform.native());
    }

    fn set_alpha(&mut self, alpha: f64) {
        self.context.set_alpha(alpha);
    }

    fn set_antialias(&mut self, enabled: bool) {
        self.context.set_antialias(enabled);
    }

    fn clip(&mut self, path: &Path, rule: FillRule) {
        self.add_path(path);
        self.context.clip(rule.native());
    }

    fn clear(&mut self, rect: Rect, color: Color) {
        self.context.clear_rect(rect.native(), color.native());
    }

    fn fill(&mut self, path: &Path, paint: &Paint, rule: FillRule) {
        self.add_path(path);
        match paint {
            Paint::Solid(color) => {
                self.context.set_fill_color(color.native());
                self.context.fill_path(rule.native());
            }
            _ => self.paint_gradient(paint, rule),
        }
    }

    fn stroke(&mut self, path: &Path, paint: &Paint, style: &StrokeStyle) {
        self.add_path(path);
        self.context.save();
        self.apply_stroke(style);
        match paint {
            Paint::Solid(color) => {
                self.context.set_stroke_color(color.native());
                self.context.stroke_path();
            }
            _ => {
                // Gradients fill the outline that the stroke would cover.
                self.context.replace_path_with_stroked_path();
                self.paint_gradient(paint, FillRule::NonZero);
            }
        }
        self.context.restore();
    }

    fn fill_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        self.context.draw_text(
            text,
            origin.native(),
            self.resources.font(font),
            color.native(),
        );
    }

    fn measure_text(&mut self, text: &str, font: &Font) -> TextMetrics {
        let metrics = self.resources.font(font).measure(text);
        TextMetrics {
            width: metrics.width,
            ascent: metrics.ascent,
            descent: metrics.descent,
            leading: metrics.leading,
        }
    }

    fn draw_image(&mut self, image: &Image, rect: Rect) {
        let frame = self.resources.frame;
        let (native, used) = match self.resources.images.entry(image.id()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let Some(native) = native::Image::from_rgba8(
                    image.width() as usize,
                    image.height() as usize,
                    image.pixels(),
                ) else {
                    return;
                };
                entry.insert((native, frame))
            }
        };
        *used = frame;
        self.context.draw_image(native, rect.native());
    }
}
