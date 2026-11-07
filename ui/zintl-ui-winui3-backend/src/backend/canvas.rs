//! Connects core canvas handlers to `zpd-winui3` canvases.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use messageloop_dispatcherqueue::Sender;
use zintl_ui_backend_core::graphics::{
    Color, FillRule, Font, Graphics2D, GraphicsBackend, Image, LineCap, LineJoin, Paint, Path,
    PathElement, Point, Rect, StrokeStyle, TextMetrics, Transform,
};
use zintl_ui_backend_core::{
    CanvasEvent, CanvasHost, CanvasId, CanvasNode, Cursor, Event, Frame, Key, KeyEvent, KeyPhase,
    MainTaskSender, Modifiers, NamedKey, PointerButton, PointerEvent, PointerPhase, ScrollEvent,
};
use zpd_winui3::{CanvasDelegate, CanvasFrame, CanvasHandle, CanvasInput, DrawContext};

use super::AppError;
use super::app::{Message, MessageSender};
use super::native::RouteCell;

/// Where a canvas reports the input its handler saw.
#[derive(Clone)]
pub(super) struct InputRoute {
    /// Whether input is reported as [`Event::Canvas`].
    pub(super) reports_input: Rc<Cell<bool>>,
    pub(super) route: RouteCell,
    pub(super) sender: MessageSender,
}

impl InputRoute {
    /// Delivers `input` to the handler and reports it; returns whether it was consumed.
    pub(super) fn deliver(&self, canvas: &CanvasNode, input: &CanvasInput) -> bool {
        let Some(event) = Self::event(input) else {
            return false;
        };
        let handled = canvas.event(&event);
        if self.reports_input.get()
            && let Some(route) = self.route.get()
        {
            let _ = self.sender.send(Message::Event {
                route,
                event: Event::Canvas(event),
            });
        }
        handled
    }

    /// Converts native input; keys that only change modifiers report them instead.
    pub(super) fn event(input: &CanvasInput) -> Option<CanvasEvent> {
        let modifiers = |native: &zpd_winui3::Modifiers| Modifiers {
            shift: native.shift,
            control: native.control,
            alt: native.alt,
            meta: native.meta,
        };
        Some(match input {
            CanvasInput::Pointer {
                phase,
                x,
                y,
                button,
                click_count,
                modifiers: native,
            } => CanvasEvent::Pointer(PointerEvent {
                phase: match phase {
                    zpd_winui3::PointerPhase::Down => PointerPhase::Down,
                    zpd_winui3::PointerPhase::Up => PointerPhase::Up,
                    zpd_winui3::PointerPhase::Moved => PointerPhase::Moved,
                    zpd_winui3::PointerPhase::Entered => PointerPhase::Entered,
                    zpd_winui3::PointerPhase::Exited => PointerPhase::Exited,
                },
                button: button.map(|button| match button {
                    0 => PointerButton::Primary,
                    1 => PointerButton::Secondary,
                    2 => PointerButton::Middle,
                    other => PointerButton::Other(other),
                }),
                location: Point::new(*x, *y),
                modifiers: modifiers(native),
                click_count: *click_count,
            }),
            CanvasInput::Scroll {
                x,
                y,
                delta_x,
                delta_y,
                precise,
                modifiers: native,
            } => CanvasEvent::Scroll(ScrollEvent {
                location: Point::new(*x, *y),
                delta_x: *delta_x,
                delta_y: *delta_y,
                precise: *precise,
                modifiers: modifiers(native),
            }),
            CanvasInput::Key {
                down,
                key,
                code,
                character,
                text,
                repeat,
                modifiers: native,
            } => CanvasEvent::Key(KeyEvent {
                phase: if *down { KeyPhase::Down } else { KeyPhase::Up },
                key: Self::key(*key, *character),
                text: text.clone(),
                modifiers: modifiers(native),
                repeat: *repeat,
                code: *code,
            }),
            CanvasInput::Text {
                text,
                modifiers: native,
            } => CanvasEvent::Key(KeyEvent {
                phase: KeyPhase::Down,
                key: Key::Unidentified,
                text: Some(text.clone()),
                modifiers: modifiers(native),
                repeat: false,
                code: 0,
            }),
            CanvasInput::Modifiers(native) => CanvasEvent::ModifiersChanged(modifiers(native)),
            CanvasInput::Focus(focused) => CanvasEvent::Focus(*focused),
        })
    }

    /// Maps a Windows virtual key and its unmodified character.
    fn key(key: u32, character: Option<char>) -> Key {
        let named = match key {
            0x0D => NamedKey::Enter,
            0x09 => NamedKey::Tab,
            0x20 => NamedKey::Space,
            0x08 => NamedKey::Backspace,
            0x2E => NamedKey::Delete,
            0x1B => NamedKey::Escape,
            0x25 => NamedKey::ArrowLeft,
            0x26 => NamedKey::ArrowUp,
            0x27 => NamedKey::ArrowRight,
            0x28 => NamedKey::ArrowDown,
            0x24 => NamedKey::Home,
            0x23 => NamedKey::End,
            0x21 => NamedKey::PageUp,
            0x22 => NamedKey::PageDown,
            0x70..=0x87 => NamedKey::F((key - 0x6F) as u8),
            _ => {
                return character.map_or(Key::Unidentified, |character| {
                    Key::Character(character.to_string())
                });
            }
        };
        Key::Named(named)
    }

    pub(super) fn frame(frame: &CanvasFrame) -> Frame {
        let inset = frame.title_bar_inset.clamp(0.0, frame.height);
        Frame {
            bounds: Rect::from_size(frame.width, frame.height),
            safe_area: Rect::new(0.0, inset, frame.width, frame.height - inset),
            scale: frame.scale,
            time: frame.time,
            delta: frame.delta,
        }
    }
}

/// Draws and routes input for one mounted Direct2D canvas.
pub(super) struct CanvasBridge {
    canvas: CanvasNode,
    input: InputRoute,
}

impl CanvasBridge {
    /// Installs `canvas` on `view` and connects its host operations.
    pub(super) fn install(
        view: &zpd_winui3::Canvas,
        canvas: &CanvasNode,
        input: InputRoute,
        main_tasks: Arc<MainTaskSender>,
    ) -> Result<(), AppError> {
        canvas.attach(Rc::new(Host::new(view, &input)?), Some(main_tasks));
        view.set_delegate(Rc::new(Self {
            canvas: canvas.clone(),
            input,
        }))?;
        Ok(())
    }
}

impl CanvasDelegate for CanvasBridge {
    fn draw(&self, context: &mut DrawContext<'_>, frame: &CanvasFrame) {
        let frame = InputRoute::frame(frame);
        let mut backend = Direct2D { context };
        let mut graphics = Graphics2D::new(&mut backend, frame.bounds, frame.scale);
        self.canvas.draw(&mut graphics, &frame);
    }

    fn input(&self, input: &CanvasInput) -> bool {
        self.input.deliver(&self.canvas, input)
    }

    fn title_bar_hit(&self, x: f64, y: f64) -> bool {
        self.canvas.title_bar_hit(Point::new(x, y))
    }
}

/// Performs a canvas's native requests through a weak handle to its view.
pub(super) struct Host {
    handle: CanvasHandle,
    route: RouteCell,
    sender: MessageSender,
}

impl Host {
    pub(super) fn new(view: &zpd_winui3::Canvas, input: &InputRoute) -> Result<Self, AppError> {
        Ok(Self {
            handle: view.handle()?,
            route: input.route.clone(),
            sender: input.sender.clone(),
        })
    }
}

impl CanvasHost for Host {
    fn invalidate(&self) {
        self.handle.invalidate();
    }

    fn set_focus(&self, focused: bool) {
        self.handle.set_focus(focused);
    }

    fn set_cursor(&self, cursor: Cursor) {
        self.handle.set_cursor(match cursor {
            Cursor::Arrow => zpd_winui3::Cursor::Arrow,
            Cursor::PointingHand => zpd_winui3::Cursor::Hand,
            Cursor::IBeam => zpd_winui3::Cursor::IBeam,
            Cursor::Crosshair => zpd_winui3::Cursor::Crosshair,
            Cursor::OpenHand => zpd_winui3::Cursor::OpenHand,
            Cursor::ClosedHand => zpd_winui3::Cursor::ClosedHand,
            Cursor::ResizeLeftRight => zpd_winui3::Cursor::ResizeLeftRight,
            Cursor::ResizeUpDown => zpd_winui3::Cursor::ResizeUpDown,
            Cursor::NotAllowed => zpd_winui3::Cursor::NotAllowed,
        });
    }

    fn actions_emitted(&self, canvas: CanvasId) {
        if let Some(route) = self.route.get() {
            let _ = self.sender.send(Message::Event {
                route,
                event: Event::CanvasActions { canvas },
            });
        }
    }
}

/// [`GraphicsBackend`] over a Direct2D frame.
struct Direct2D<'a, 'frame> {
    context: &'a mut DrawContext<'frame>,
}

impl Direct2D<'_, '_> {
    fn color(color: Color) -> zpd_winui3::Color {
        zpd_winui3::Color {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha: color.alpha,
        }
    }

    fn elements(path: &Path) -> Vec<zpd_winui3::PathElement> {
        path.elements()
            .iter()
            .map(|element| match *element {
                PathElement::MoveTo(point) => zpd_winui3::PathElement::MoveTo(point.x, point.y),
                PathElement::LineTo(point) => zpd_winui3::PathElement::LineTo(point.x, point.y),
                PathElement::QuadTo(control, point) => {
                    zpd_winui3::PathElement::QuadTo(control.x, control.y, point.x, point.y)
                }
                PathElement::CubicTo(first, second, point) => zpd_winui3::PathElement::CubicTo(
                    first.x, first.y, second.x, second.y, point.x, point.y,
                ),
                PathElement::Close => zpd_winui3::PathElement::Close,
            })
            .collect()
    }

    /// Calls `body` with the native path, whose elements live for the call.
    fn with_path<R>(
        path: &Path,
        rule: FillRule,
        body: impl FnOnce(&zpd_winui3::Path<'_>) -> R,
    ) -> R {
        let elements = Self::elements(path);
        body(&zpd_winui3::Path {
            elements: &elements,
            even_odd: rule == FillRule::EvenOdd,
        })
    }

    /// Calls `body` with the native paint, whose stops live for the call.
    fn with_paint<R>(paint: &Paint, body: impl FnOnce(&zpd_winui3::Paint<'_>) -> R) -> R {
        let stops = |stops: &[zintl_ui_backend_core::graphics::GradientStop]| {
            stops
                .iter()
                .map(|stop| zpd_winui3::GradientStop {
                    offset: stop.offset,
                    color: Self::color(stop.color),
                })
                .collect::<Vec<_>>()
        };
        match paint {
            Paint::Solid(color) => body(&zpd_winui3::Paint::Solid(Self::color(*color))),
            Paint::Linear(gradient) => {
                let stops = stops(&gradient.stops);
                body(&zpd_winui3::Paint::Linear {
                    start: (gradient.start.x, gradient.start.y),
                    end: (gradient.end.x, gradient.end.y),
                    stops: &stops,
                })
            }
            Paint::Radial(gradient) => {
                let stops = stops(&gradient.stops);
                body(&zpd_winui3::Paint::Radial {
                    center: (gradient.center.x, gradient.center.y),
                    radius: gradient.radius,
                    stops: &stops,
                })
            }
        }
    }

    fn font(font: &Font) -> zpd_winui3::Font<'_> {
        zpd_winui3::Font {
            family: font.family.as_deref(),
            size: font.size(),
            weight: font.weight.0,
            italic: font.italic,
        }
    }

    fn rect(rect: Rect) -> zpd_winui3::Rect {
        zpd_winui3::Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}

impl GraphicsBackend for Direct2D<'_, '_> {
    fn save(&mut self) {
        self.context.save();
    }

    fn restore(&mut self) {
        self.context.restore();
    }

    fn concat(&mut self, transform: Transform) {
        self.context.concat(zpd_winui3::Transform {
            a: transform.a,
            b: transform.b,
            c: transform.c,
            d: transform.d,
            tx: transform.tx,
            ty: transform.ty,
        });
    }

    fn set_alpha(&mut self, alpha: f64) {
        self.context.set_alpha(alpha);
    }

    fn set_antialias(&mut self, enabled: bool) {
        self.context.set_antialias(enabled);
    }

    fn clip(&mut self, path: &Path, rule: FillRule) {
        Self::with_path(path, rule, |path| self.context.clip(path));
    }

    fn clear(&mut self, rect: Rect, color: Color) {
        self.context.clear(Self::rect(rect), Self::color(color));
    }

    fn fill(&mut self, path: &Path, paint: &Paint, rule: FillRule) {
        Self::with_path(path, rule, |path| {
            Self::with_paint(paint, |paint| self.context.fill(path, paint))
        });
    }

    fn stroke(&mut self, path: &Path, paint: &Paint, style: &StrokeStyle) {
        let stroke = zpd_winui3::Stroke {
            width: style.width,
            cap: match style.cap {
                LineCap::Butt => zpd_winui3::LineCap::Butt,
                LineCap::Round => zpd_winui3::LineCap::Round,
                LineCap::Square => zpd_winui3::LineCap::Square,
            },
            join: match style.join {
                LineJoin::Miter => zpd_winui3::LineJoin::Miter,
                LineJoin::Round => zpd_winui3::LineJoin::Round,
                LineJoin::Bevel => zpd_winui3::LineJoin::Bevel,
            },
            miter_limit: style.miter_limit,
            dashes: &style.dashes,
            dash_offset: style.dash_offset,
        };
        Self::with_path(path, FillRule::NonZero, |path| {
            Self::with_paint(paint, |paint| self.context.stroke(path, paint, &stroke))
        });
    }

    fn fill_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        self.context.fill_text(
            text,
            origin.x,
            origin.y,
            &Self::font(font),
            Self::color(color),
        );
    }

    fn measure_text(&mut self, text: &str, font: &Font) -> TextMetrics {
        // A font DirectWrite cannot measure takes no space.
        zpd_winui3::measure_text(text, &Self::font(font))
            .map(|metrics| TextMetrics {
                width: metrics.width,
                ascent: metrics.ascent,
                descent: metrics.descent,
                leading: metrics.leading,
            })
            .unwrap_or_default()
    }

    fn draw_image(&mut self, image: &Image, rect: Rect) {
        self.context.draw_image(
            zpd_winui3::ImageData {
                id: image.id(),
                width: image.width(),
                height: image.height(),
                pixels: image.pixels(),
            },
            Self::rect(rect),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_keys_map_to_named_keys_or_characters() {
        // Verifies navigation and function keys are named and other keys use their character.
        assert_eq!(InputRoute::key(0x0D, None), Key::Named(NamedKey::Enter));
        assert_eq!(InputRoute::key(0x71, None), Key::Named(NamedKey::F(2)));
        assert_eq!(InputRoute::key(0x41, Some('a')), Key::Character("a".into()));
        assert_eq!(InputRoute::key(0xFF, None), Key::Unidentified);
    }

    #[test]
    fn text_without_a_key_arrives_as_an_unidentified_key_press() {
        // Verifies emoji panel text reaches handlers as a key press carrying the text.
        let event = InputRoute::event(&CanvasInput::Text {
            text: "😀".into(),
            modifiers: zpd_winui3::Modifiers::default(),
        });
        let Some(CanvasEvent::Key(key)) = event else {
            panic!("text should become a key event");
        };
        assert_eq!(
            (key.phase, key.key, key.text.as_deref()),
            (KeyPhase::Down, Key::Unidentified, Some("😀"))
        );
    }

    #[test]
    fn the_safe_area_excludes_the_title_bar_inset() {
        // Verifies a canvas under the title bar reports the uncovered part as its safe area.
        let frame = InputRoute::frame(&CanvasFrame {
            width: 100.0,
            height: 50.0,
            scale: 1.5,
            time: 2.0,
            delta: 0.0,
            title_bar_inset: 32.0,
        });
        assert_eq!(frame.safe_area, Rect::new(0.0, 32.0, 100.0, 18.0));
        assert_eq!(frame.bounds, Rect::from_size(100.0, 50.0));
    }
}
