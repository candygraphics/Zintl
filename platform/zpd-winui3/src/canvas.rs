//! Canvases that Direct2D draws, or that another renderer such as wgpu presents to.

use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

use super::{
    AsElement, Color, ElementRef, Error, OwnedElement, Result, abort_on_panic, check, ffi,
};

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasMode {
    /// Each frame is drawn through a [`DrawContext`].
    Direct2D = 0,
    /// Another renderer presents to [`Canvas::swap_chain_panel`].
    External = 1,
}

/// Geometry and timing of one frame, in DIPs and seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasFrame {
    pub width: f64,
    pub height: f64,
    /// Device pixels per DIP.
    pub scale: f64,
    /// Seconds on a monotonic clock.
    pub time: f64,
    /// Seconds since the previous frame, or `0.0` for the first one.
    pub delta: f64,
    /// The height of the canvas's top part under the window's title bar.
    pub title_bar_inset: f64,
}

/// Modifier keys held during input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    /// The Windows key.
    pub meta: bool,
}

impl Modifiers {
    fn from_native(bits: u32) -> Self {
        Self {
            shift: bits & 1 != 0,
            control: bits & 2 != 0,
            alt: bits & 4 != 0,
            meta: bits & 8 != 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Down,
    Up,
    Moved,
    Entered,
    Exited,
}

/// Input a canvas receives, in DIPs from its top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasInput {
    Pointer {
        phase: PointerPhase,
        x: f64,
        y: f64,
        /// 0 is the primary button, 1 the secondary, and 2 the middle one.
        button: Option<u8>,
        click_count: u32,
        modifiers: Modifiers,
    },
    /// Distances in DIPs; positive values move content right and down.
    Scroll {
        x: f64,
        y: f64,
        delta_x: f64,
        delta_y: f64,
        precise: bool,
        modifiers: Modifiers,
    },
    Key {
        down: bool,
        /// The Windows virtual key.
        key: u32,
        /// The hardware scan code.
        code: u32,
        /// The character the key produces without modifiers.
        character: Option<char>,
        /// The text the press inserts with its modifiers.
        text: Option<String>,
        repeat: bool,
        modifiers: Modifiers,
    },
    /// Text that no key press produced, such as from the emoji panel.
    Text {
        text: String,
        modifiers: Modifiers,
    },
    Modifiers(Modifiers),
    Focus(bool),
}

impl CanvasInput {
    /// Converts native input; `None` marks an unknown kind.
    ///
    /// # Safety
    /// `input.text` must point to valid UTF-8 for the duration of the call.
    unsafe fn from_native(input: &ffi::CanvasInput) -> Option<Self> {
        let modifiers = Modifiers::from_native(input.modifiers);
        // SAFETY: The caller guarantees the text buffer for this synchronous call.
        let text = || unsafe { super::string_from_ref(input.text) };
        let button = u8::try_from(input.button).ok();
        let pointer = |phase| Self::Pointer {
            phase,
            x: input.x,
            y: input.y,
            button,
            click_count: input.click_count,
            modifiers,
        };
        Some(match input.kind {
            0 => pointer(PointerPhase::Down),
            1 => pointer(PointerPhase::Up),
            2 => pointer(PointerPhase::Moved),
            3 => pointer(PointerPhase::Entered),
            4 => pointer(PointerPhase::Exited),
            5 => Self::Scroll {
                x: input.x,
                y: input.y,
                delta_x: input.delta_x,
                delta_y: input.delta_y,
                precise: input.precise,
                modifiers,
            },
            6 | 7 => Self::Key {
                down: input.kind == 6,
                key: input.key,
                code: input.code,
                character: char::from_u32(input.character).filter(|character| *character != '\0'),
                text: Some(text()).filter(|text| !text.is_empty()),
                repeat: input.repeat,
                modifiers,
            },
            8 => Self::Text {
                text: text(),
                modifiers,
            },
            9 => Self::Modifiers(modifiers),
            10 => Self::Focus(input.flag),
            _ => return None,
        })
    }
}

/// Draws and receives the input of a [`Canvas`]; every method runs on the UI thread.
pub trait CanvasDelegate {
    /// Draws a [`CanvasMode::Direct2D`] frame.
    fn draw(&self, context: &mut DrawContext<'_>, frame: &CanvasFrame) {
        let _ = (context, frame);
    }

    /// Renders a [`CanvasMode::External`] frame.
    fn render(&self, frame: &CanvasFrame) {
        let _ = frame;
    }

    /// Returns whether the input was consumed.
    fn input(&self, input: &CanvasInput) -> bool;

    /// Returns whether a press at canvas point `(x, y)` under the title bar
    /// belongs to the canvas; otherwise it moves the window.
    fn title_bar_hit(&self, x: f64, y: f64) -> bool {
        let _ = (x, y);
        false
    }
}

/// The pointer shape shown over a canvas.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Arrow = 0,
    Hand = 1,
    IBeam = 2,
    Crosshair = 3,
    OpenHand = 4,
    ClosedHand = 5,
    ResizeLeftRight = 6,
    ResizeUpDown = 7,
    NotAllowed = 8,
}

/// A `SwapChainPanel` that a delegate draws.
pub struct Canvas {
    element: OwnedElement,
}

impl AsElement for Canvas {
    fn as_element(&self) -> ElementRef<'_> {
        self.element.as_element()
    }
}

impl Canvas {
    pub fn new(mode: CanvasMode) -> Result<Self> {
        // SAFETY: Native creation runs on the UI thread and returns an owned wrapper.
        let raw = unsafe { ffi::zpd_winui3_canvas_create(mode as i32) };
        Ok(Self {
            // SAFETY: A non-null result is a newly owned canvas wrapper.
            element: unsafe { OwnedElement::from_raw(raw)? },
        })
    }

    fn raw(&self) -> *mut ffi::Element {
        self.element.raw.as_ptr()
    }

    /// Installs the delegate, replacing any earlier one, and schedules a draw.
    pub fn set_delegate(&self, delegate: Rc<dyn CanvasDelegate>) -> Result<()> {
        type Data = Rc<dyn CanvasDelegate>;

        /// Clones the delegate so a replacement during the call cannot free it.
        ///
        /// # Safety
        /// `data` must be the live `Box<Data>` installed below.
        unsafe fn delegate_of(data: *const c_void) -> Data {
            // SAFETY: The caller guarantees `data` points at the installed box.
            unsafe { (*data.cast::<Data>()).clone() }
        }

        unsafe extern "C" fn draw(
            data: *const c_void,
            context: *mut ffi::Draw,
            frame: *const ffi::CanvasFrame,
        ) {
            abort_on_panic(|| {
                // SAFETY: Native code passes the installed data and a live frame.
                let (delegate, frame) = unsafe { (delegate_of(data), &*frame) };
                let frame = CanvasFrame {
                    width: frame.width,
                    height: frame.height,
                    scale: frame.scale,
                    time: frame.time,
                    delta: frame.delta,
                    title_bar_inset: frame.title_bar_inset,
                };
                match NonNull::new(context) {
                    Some(raw) => delegate.draw(
                        &mut DrawContext {
                            raw,
                            _frame: PhantomData,
                        },
                        &frame,
                    ),
                    None => delegate.render(&frame),
                }
            });
        }

        unsafe extern "C" fn input(data: *const c_void, input: *const ffi::CanvasInput) -> bool {
            let mut handled = false;
            abort_on_panic(|| {
                // SAFETY: Native code passes the installed data and a live input record.
                let delegate = unsafe { delegate_of(data) };
                // SAFETY: The input's text is valid for this synchronous call.
                if let Some(input) = unsafe { CanvasInput::from_native(&*input) } {
                    handled = delegate.input(&input);
                }
            });
            handled
        }

        unsafe extern "C" fn title_bar_hit(data: *const c_void, x: f64, y: f64) -> bool {
            let mut hit = false;
            abort_on_panic(|| {
                // SAFETY: Native code passes the installed data.
                hit = unsafe { delegate_of(data) }.title_bar_hit(x, y);
            });
            hit
        }

        unsafe extern "C" fn release(data: *const c_void) {
            if !data.is_null() {
                // SAFETY: Native code releases the one box transferred to it exactly once.
                unsafe { drop(Box::from_raw(data.cast_mut().cast::<Data>())) };
            }
        }

        let data = Box::into_raw(Box::new(delegate));
        // SAFETY: Native code takes ownership of `data`, including when installation fails.
        check(unsafe {
            ffi::zpd_winui3_canvas_set_callbacks(
                self.raw(),
                data.cast(),
                ffi::CanvasCallbacks {
                    draw,
                    input,
                    title_bar_hit,
                },
                release,
            )
        })
    }

    /// Draws every display refresh, at most `fps` times a second, or only on request.
    pub fn set_redraw(&self, continuous: bool, fps: Option<f64>) -> Result<()> {
        // SAFETY: The canvas is live on the UI thread.
        check(unsafe {
            ffi::zpd_winui3_canvas_set_redraw(self.raw(), continuous, fps.unwrap_or(0.0))
        })
    }

    /// Lets a click or [`CanvasHandle::set_focus`] give the canvas keyboard focus.
    pub fn set_focusable(&self, focusable: bool) -> Result<()> {
        // SAFETY: The canvas is live on the UI thread.
        check(unsafe { ffi::zpd_winui3_canvas_set_focusable(self.raw(), focusable) })
    }

    /// Returns a weak handle for requests made while drawing or handling input.
    pub fn handle(&self) -> Result<CanvasHandle> {
        // SAFETY: The canvas is live on the UI thread.
        let raw = unsafe { ffi::zpd_winui3_canvas_handle_create(self.raw()) };
        Ok(CanvasHandle {
            raw: NonNull::new(raw).ok_or(Error(-1))?,
        })
    }

    /// Returns the `ISwapChainPanelNative` of an external canvas, or null.
    ///
    /// It stays valid while the canvas lives; a swap chain set through it is
    /// scaled so its pixels cover the canvas in DIPs.
    #[must_use]
    pub fn swap_chain_panel(&self) -> *mut c_void {
        // SAFETY: The canvas is live on the UI thread.
        unsafe { ffi::zpd_winui3_canvas_swap_chain_panel(self.raw()) }
    }
}

/// A weak reference to a [`Canvas`]; requests do nothing once it is gone.
pub struct CanvasHandle {
    raw: NonNull<ffi::CanvasHandle>,
}

impl CanvasHandle {
    /// Schedules one draw; requests before it runs are coalesced.
    pub fn invalidate(&self) {
        // SAFETY: The handle is live on the UI thread.
        unsafe { ffi::zpd_winui3_canvas_handle_invalidate(self.raw.as_ptr()) }
    }

    pub fn set_focus(&self, focused: bool) {
        // SAFETY: The handle is live on the UI thread.
        unsafe { ffi::zpd_winui3_canvas_handle_set_focus(self.raw.as_ptr(), focused) }
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        // SAFETY: The handle is live on the UI thread.
        unsafe { ffi::zpd_winui3_canvas_handle_set_cursor(self.raw.as_ptr(), cursor as i32) }
    }
}

impl Drop for CanvasHandle {
    fn drop(&mut self) {
        // SAFETY: This owned handle is released exactly once.
        unsafe { ffi::zpd_winui3_canvas_handle_release(self.raw.as_ptr()) };
    }
}

/// One element of a path; points are in DIPs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathElement {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    /// A control point, then the end point.
    QuadTo(f64, f64, f64, f64),
    /// Two control points, then the end point.
    CubicTo(f64, f64, f64, f64, f64, f64),
    Close,
}

impl PathElement {
    fn native(self) -> ffi::PathElement {
        let (verb, points) = match self {
            Self::MoveTo(x, y) => (0, [x, y, 0.0, 0.0, 0.0, 0.0]),
            Self::LineTo(x, y) => (1, [x, y, 0.0, 0.0, 0.0, 0.0]),
            Self::QuadTo(x1, y1, x, y) => (2, [x1, y1, x, y, 0.0, 0.0]),
            Self::CubicTo(x1, y1, x2, y2, x, y) => (3, [x1, y1, x2, y2, x, y]),
            Self::Close => (4, [0.0; 6]),
        };
        ffi::PathElement { verb, points }
    }
}

/// A path and how its inside is decided.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Path<'a> {
    pub elements: &'a [PathElement],
    /// Even-odd instead of non-zero winding.
    pub even_odd: bool,
}

impl Path<'_> {
    /// Calls `body` with the native path, whose storage lives for the call.
    fn with_native<R>(&self, body: impl FnOnce(&ffi::Path) -> R) -> R {
        let elements: Vec<_> = self
            .elements
            .iter()
            .map(|element| element.native())
            .collect();
        body(&ffi::Path {
            elements: elements.as_ptr(),
            length: elements.len(),
            even_odd: self.even_odd,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint<'a> {
    Solid(Color),
    Linear {
        start: (f64, f64),
        end: (f64, f64),
        stops: &'a [GradientStop],
    },
    Radial {
        center: (f64, f64),
        radius: f64,
        stops: &'a [GradientStop],
    },
}

impl Paint<'_> {
    /// Calls `body` with the native paint, whose storage lives for the call.
    fn with_native<R>(&self, body: impl FnOnce(&ffi::Paint) -> R) -> R {
        let (kind, color, (x0, y0), (x1, y1), radius, stops) = match *self {
            Self::Solid(color) => (0, color, (0.0, 0.0), (0.0, 0.0), 0.0, &[][..]),
            Self::Linear { start, end, stops } => (1, Color::default(), start, end, 0.0, stops),
            Self::Radial {
                center,
                radius,
                stops,
            } => (2, Color::default(), center, (0.0, 0.0), radius, stops),
        };
        let stops: Vec<_> = stops
            .iter()
            .map(|stop| ffi::GradientStop {
                offset: stop.offset,
                color: stop.color.native(),
            })
            .collect();
        body(&ffi::Paint {
            kind,
            color: color.native(),
            x0,
            y0,
            x1,
            y1,
            radius,
            stops: stops.as_ptr(),
            stops_length: stops.len(),
        })
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineCap {
    #[default]
    Butt = 0,
    Round = 1,
    Square = 2,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineJoin {
    #[default]
    Miter = 0,
    Round = 1,
    Bevel = 2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke<'a> {
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    /// Alternating dash and gap lengths in DIPs; empty draws a solid line.
    pub dashes: &'a [f64],
    pub dash_offset: f64,
}

/// A DirectWrite font description.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Font<'a> {
    /// `None` uses the system font.
    pub family: Option<&'a str>,
    pub size: f64,
    /// A CSS-style weight such as 400 or 700.
    pub weight: u16,
    pub italic: bool,
}

impl Font<'_> {
    fn native(&self) -> ffi::Font {
        ffi::Font {
            family: ffi::StringRef::new(self.family.unwrap_or("")),
            size: self.size,
            weight: i32::from(self.weight),
            italic: self.italic,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub width: f64,
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
}

/// Measures one line of text.
pub fn measure_text(text: &str, font: &Font<'_>) -> Result<TextMetrics> {
    let mut metrics = ffi::TextMetrics::default();
    let font = font.native();
    // SAFETY: The strings are borrowed and `metrics` outlives the call.
    check(unsafe {
        ffi::zpd_winui3_measure_text(ffi::StringRef::new(text), &raw const font, &raw mut metrics)
    })?;
    Ok(TextMetrics {
        width: metrics.width,
        ascent: metrics.ascent,
        descent: metrics.descent,
        leading: metrics.leading,
    })
}

/// An affine transform mapping `(x, y)` to `(a*x + c*y + tx, b*x + d*y + ty)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

/// A rectangle in DIPs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Straight-alpha RGBA8 pixels and an identity that caches the native bitmap.
#[derive(Clone, Copy, Debug)]
pub struct ImageData<'a> {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],
}

/// Direct2D drawing for the frame being drawn; state is scoped by `save` and `restore`.
pub struct DrawContext<'frame> {
    raw: NonNull<ffi::Draw>,
    _frame: PhantomData<&'frame mut ()>,
}

impl DrawContext<'_> {
    fn raw(&mut self) -> *mut ffi::Draw {
        self.raw.as_ptr()
    }

    pub fn save(&mut self) {
        // SAFETY: The context is live during the frame.
        unsafe { ffi::zpd_winui3_draw_save(self.raw()) }
    }

    pub fn restore(&mut self) {
        // SAFETY: The context is live during the frame.
        unsafe { ffi::zpd_winui3_draw_restore(self.raw()) }
    }

    /// Applies `transform` before the current transform.
    pub fn concat(&mut self, transform: Transform) {
        let native = ffi::Transform {
            a: transform.a,
            b: transform.b,
            c: transform.c,
            d: transform.d,
            tx: transform.tx,
            ty: transform.ty,
        };
        // SAFETY: The context is live during the frame.
        unsafe { ffi::zpd_winui3_draw_concat(self.raw(), native) }
    }

    pub fn set_alpha(&mut self, alpha: f64) {
        // SAFETY: The context is live during the frame.
        unsafe { ffi::zpd_winui3_draw_set_alpha(self.raw(), alpha) }
    }

    pub fn set_antialias(&mut self, enabled: bool) {
        // SAFETY: The context is live during the frame.
        unsafe { ffi::zpd_winui3_draw_set_antialias(self.raw(), enabled) }
    }

    /// Intersects the clip with `path` until the state is restored.
    pub fn clip(&mut self, path: &Path<'_>) {
        let raw = self.raw();
        // SAFETY: The context is live and the path storage outlives the call.
        path.with_native(|path| unsafe { ffi::zpd_winui3_draw_clip(raw, path) });
    }

    /// Replaces the pixels of `rect`, including their alpha, with `color`.
    pub fn clear(&mut self, rect: Rect, color: Color) {
        // SAFETY: The context is live during the frame.
        unsafe {
            ffi::zpd_winui3_draw_clear(
                self.raw(),
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                color.native(),
            )
        }
    }

    pub fn fill(&mut self, path: &Path<'_>, paint: &Paint<'_>) {
        let raw = self.raw();
        path.with_native(|path| {
            // SAFETY: The context is live and both storages outlive the call.
            paint.with_native(|paint| unsafe { ffi::zpd_winui3_draw_fill(raw, path, paint) })
        });
    }

    pub fn stroke(&mut self, path: &Path<'_>, paint: &Paint<'_>, stroke: &Stroke<'_>) {
        let raw = self.raw();
        let native = ffi::Stroke {
            width: stroke.width,
            cap: stroke.cap as i32,
            join: stroke.join as i32,
            miter_limit: stroke.miter_limit,
            dashes: stroke.dashes.as_ptr(),
            dashes_length: stroke.dashes.len(),
            dash_offset: stroke.dash_offset,
        };
        path.with_native(|path| {
            paint.with_native(|paint| {
                // SAFETY: The context is live and every storage outlives the call.
                unsafe { ffi::zpd_winui3_draw_stroke(raw, path, paint, &raw const native) }
            })
        });
    }

    /// Draws one line of text whose baseline starts at `(x, y)`.
    pub fn fill_text(&mut self, text: &str, x: f64, y: f64, font: &Font<'_>, color: Color) {
        let font = font.native();
        // SAFETY: The context is live and the strings outlive the call.
        unsafe {
            ffi::zpd_winui3_draw_text(
                self.raw(),
                ffi::StringRef::new(text),
                x,
                y,
                &raw const font,
                color.native(),
            )
        }
    }

    /// Draws `image` scaled into `rect`; the bitmap is cached while frames keep drawing it.
    pub fn draw_image(&mut self, image: ImageData<'_>, rect: Rect) {
        if image.pixels.len() != image.width as usize * image.height as usize * 4 {
            return;
        }
        // SAFETY: The context is live and the pixels hold `width * height` RGBA values.
        unsafe {
            ffi::zpd_winui3_draw_image(
                self.raw(),
                image.id,
                image.width,
                image.height,
                image.pixels.as_ptr(),
                rect.x,
                rect.y,
                rect.width,
                rect.height,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native(kind: i32) -> ffi::CanvasInput {
        ffi::CanvasInput {
            kind,
            x: 1.0,
            y: 2.0,
            button: -1,
            modifiers: 0b0101,
            click_count: 2,
            delta_x: 0.0,
            delta_y: -48.0,
            precise: false,
            key: 0x41,
            code: 30,
            character: u32::from('a'),
            repeat: false,
            text: ffi::StringRef::new("A"),
            flag: true,
        }
    }

    #[test]
    fn native_input_maps_to_its_kind() {
        // Verifies the C++ input kinds, buttons, and modifier bits decode as documented.
        // SAFETY: Every test input points at a live string literal.
        let convert = |kind| unsafe { CanvasInput::from_native(&native(kind)) };
        let modifiers = Modifiers {
            shift: true,
            alt: true,
            ..Modifiers::default()
        };
        assert_eq!(
            convert(0),
            Some(CanvasInput::Pointer {
                phase: PointerPhase::Down,
                x: 1.0,
                y: 2.0,
                button: None,
                click_count: 2,
                modifiers,
            })
        );
        assert_eq!(
            convert(6),
            Some(CanvasInput::Key {
                down: true,
                key: 0x41,
                code: 30,
                character: Some('a'),
                text: Some("A".into()),
                repeat: false,
                modifiers,
            })
        );
        assert_eq!(convert(10), Some(CanvasInput::Focus(true)));
        assert_eq!(convert(99), None);
    }

    #[test]
    fn path_elements_put_the_end_point_last() {
        // Verifies the native path layout the C++ geometry builder reads.
        let cubic = PathElement::CubicTo(1.0, 2.0, 3.0, 4.0, 5.0, 6.0).native();
        assert_eq!(
            (cubic.verb, cubic.points),
            (3, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
        );
        let quad = PathElement::QuadTo(1.0, 2.0, 3.0, 4.0).native();
        assert_eq!(
            (quad.verb, &quad.points[..4]),
            (2, &[1.0, 2.0, 3.0, 4.0][..])
        );
    }
}
