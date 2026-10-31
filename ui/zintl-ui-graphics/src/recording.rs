use crate::{
    Color, FillRule, Font, GraphicsBackend, Image, Paint, Path, Point, Rect, StrokeStyle,
    TextMetrics, Transform,
};

/// A drawing operation captured by [`RecordingBackend`].
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    Save,
    Restore,
    Concat(Transform),
    SetAlpha(f64),
    SetAntialias(bool),
    Clip {
        path: Path,
        rule: FillRule,
    },
    Clear {
        rect: Rect,
        color: Color,
    },
    Fill {
        path: Path,
        paint: Paint,
        rule: FillRule,
    },
    Stroke {
        path: Path,
        paint: Paint,
        style: StrokeStyle,
    },
    FillText {
        text: String,
        origin: Point,
        font: Font,
        color: Color,
    },
    DrawImage {
        image: Image,
        rect: Rect,
    },
}

/// Records drawing instead of rasterizing it, for tests and headless rendering.
///
/// Text metrics are estimated from the font size because no font is loaded.
#[derive(Debug, Default)]
pub struct RecordingBackend {
    commands: Vec<DrawCommand>,
}

impl RecordingBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn into_commands(self) -> Vec<DrawCommand> {
        self.commands
    }
}

impl GraphicsBackend for RecordingBackend {
    fn save(&mut self) {
        self.commands.push(DrawCommand::Save);
    }

    fn restore(&mut self) {
        self.commands.push(DrawCommand::Restore);
    }

    fn concat(&mut self, transform: Transform) {
        self.commands.push(DrawCommand::Concat(transform));
    }

    fn set_alpha(&mut self, alpha: f64) {
        self.commands.push(DrawCommand::SetAlpha(alpha));
    }

    fn set_antialias(&mut self, enabled: bool) {
        self.commands.push(DrawCommand::SetAntialias(enabled));
    }

    fn clip(&mut self, path: &Path, rule: FillRule) {
        self.commands.push(DrawCommand::Clip {
            path: path.clone(),
            rule,
        });
    }

    fn clear(&mut self, rect: Rect, color: Color) {
        self.commands.push(DrawCommand::Clear { rect, color });
    }

    fn fill(&mut self, path: &Path, paint: &Paint, rule: FillRule) {
        self.commands.push(DrawCommand::Fill {
            path: path.clone(),
            paint: paint.clone(),
            rule,
        });
    }

    fn stroke(&mut self, path: &Path, paint: &Paint, style: &StrokeStyle) {
        self.commands.push(DrawCommand::Stroke {
            path: path.clone(),
            paint: paint.clone(),
            style: style.clone(),
        });
    }

    fn fill_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        self.commands.push(DrawCommand::FillText {
            text: text.to_owned(),
            origin,
            font: font.clone(),
            color,
        });
    }

    fn measure_text(&mut self, text: &str, font: &Font) -> TextMetrics {
        TextMetrics {
            width: text.chars().count() as f64 * font.size() * 0.5,
            ascent: font.size() * 0.8,
            descent: font.size() * 0.2,
            leading: 0.0,
        }
    }

    fn draw_image(&mut self, image: &Image, rect: Rect) {
        self.commands.push(DrawCommand::DrawImage {
            image: image.clone(),
            rect,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Graphics2D;

    #[test]
    fn save_scopes_are_balanced() {
        // Verifies Graphics2D::save restores the state after the closure returns.
        let mut backend = RecordingBackend::new();
        let mut graphics = Graphics2D::new(&mut backend, Rect::from_size(10.0, 10.0), 2.0);
        graphics.save(|g| g.translate(1.0, 2.0));
        assert_eq!(
            backend.commands(),
            &[
                DrawCommand::Save,
                DrawCommand::Concat(Transform::translation(1.0, 2.0)),
                DrawCommand::Restore,
            ]
        );
    }

    #[test]
    fn colors_convert_to_solid_paint() {
        // Verifies fill helpers accept plain colors as solid paint.
        let mut backend = RecordingBackend::new();
        let rect = Rect::new(1.0, 1.0, 2.0, 2.0);
        Graphics2D::new(&mut backend, Rect::from_size(4.0, 4.0), 1.0).fill_rect(rect, Color::WHITE);
        assert_eq!(
            backend.commands(),
            &[DrawCommand::Fill {
                path: Path::rect(rect),
                paint: Paint::Solid(Color::WHITE),
                rule: FillRule::NonZero,
            }]
        );
    }
}
