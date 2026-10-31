//! Core Graphics and Core Text drawing for canvas views.
//!
//! A [`GraphicsContext`] exists only while a canvas view draws. Coordinates
//! are the view's flipped space: logical points with a top-left origin.

use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::NonNull;

use zpd_objc::{Id, Strong};

use crate::geometry::{Point, Rect};
use crate::native;
use crate::ui::Color;

type CGContextRef = *mut c_void;
type CFTypeRef = *const c_void;

/// A Core Foundation object owned with one retain count.
struct CfObject(NonNull<c_void>);

impl CfObject {
    /// # Safety
    /// `raw` must be null or a +1 Core Foundation reference.
    unsafe fn from_retained(raw: CFTypeRef) -> Option<Self> {
        NonNull::new(raw.cast_mut()).map(Self)
    }

    fn as_ptr(&self) -> CFTypeRef {
        self.0.as_ptr()
    }
}

impl Drop for CfObject {
    fn drop(&mut self) {
        // SAFETY: This releases the single reference owned by this value.
        unsafe { CFRelease(self.as_ptr()) };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGAffineTransform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: CFTypeRef);
    fn CFDataCreate(allocator: CFTypeRef, bytes: *const u8, length: isize) -> CFTypeRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    static kCGColorSpaceSRGB: CFTypeRef;

    fn CGColorSpaceCreateWithName(name: CFTypeRef) -> CFTypeRef;
    fn CGContextSaveGState(context: CGContextRef);
    fn CGContextRestoreGState(context: CGContextRef);
    fn CGContextConcatCTM(context: CGContextRef, transform: CGAffineTransform);
    fn CGContextTranslateCTM(context: CGContextRef, x: f64, y: f64);
    fn CGContextScaleCTM(context: CGContextRef, x: f64, y: f64);
    fn CGContextSetAlpha(context: CGContextRef, alpha: f64);
    fn CGContextSetShouldAntialias(context: CGContextRef, enabled: bool);
    fn CGContextSetBlendMode(context: CGContextRef, mode: i32);
    fn CGContextBeginPath(context: CGContextRef);
    fn CGContextMoveToPoint(context: CGContextRef, x: f64, y: f64);
    fn CGContextAddLineToPoint(context: CGContextRef, x: f64, y: f64);
    fn CGContextAddQuadCurveToPoint(context: CGContextRef, cx: f64, cy: f64, x: f64, y: f64);
    fn CGContextAddCurveToPoint(
        context: CGContextRef,
        c1x: f64,
        c1y: f64,
        c2x: f64,
        c2y: f64,
        x: f64,
        y: f64,
    );
    fn CGContextClosePath(context: CGContextRef);
    fn CGContextFillPath(context: CGContextRef);
    fn CGContextEOFillPath(context: CGContextRef);
    fn CGContextStrokePath(context: CGContextRef);
    fn CGContextClip(context: CGContextRef);
    fn CGContextEOClip(context: CGContextRef);
    fn CGContextReplacePathWithStrokedPath(context: CGContextRef);
    fn CGContextFillRect(context: CGContextRef, rect: native::Rect);
    fn CGContextSetFillColorSpace(context: CGContextRef, space: CFTypeRef);
    fn CGContextSetStrokeColorSpace(context: CGContextRef, space: CFTypeRef);
    fn CGContextSetFillColor(context: CGContextRef, components: *const f64);
    fn CGContextSetStrokeColor(context: CGContextRef, components: *const f64);
    fn CGContextSetLineWidth(context: CGContextRef, width: f64);
    fn CGContextSetLineCap(context: CGContextRef, cap: i32);
    fn CGContextSetLineJoin(context: CGContextRef, join: i32);
    fn CGContextSetMiterLimit(context: CGContextRef, limit: f64);
    fn CGContextSetLineDash(context: CGContextRef, phase: f64, lengths: *const f64, count: usize);
    fn CGContextSetInterpolationQuality(context: CGContextRef, quality: i32);
    fn CGContextDrawImage(context: CGContextRef, rect: native::Rect, image: CFTypeRef);
    fn CGContextDrawLinearGradient(
        context: CGContextRef,
        gradient: CFTypeRef,
        start: native::Point,
        end: native::Point,
        options: u32,
    );
    fn CGContextDrawRadialGradient(
        context: CGContextRef,
        gradient: CFTypeRef,
        start_center: native::Point,
        start_radius: f64,
        end_center: native::Point,
        end_radius: f64,
        options: u32,
    );
    fn CGContextSetTextMatrix(context: CGContextRef, transform: CGAffineTransform);
    fn CGContextSetTextPosition(context: CGContextRef, x: f64, y: f64);
    fn CGGradientCreateWithColorComponents(
        space: CFTypeRef,
        components: *const f64,
        locations: *const f64,
        count: usize,
    ) -> CFTypeRef;
    fn CGDataProviderCreateWithCFData(data: CFTypeRef) -> CFTypeRef;
    fn CGDataProviderRelease(provider: CFTypeRef);
    #[allow(clippy::too_many_arguments)]
    fn CGImageCreate(
        width: usize,
        height: usize,
        bits_per_component: usize,
        bits_per_pixel: usize,
        bytes_per_row: usize,
        space: CFTypeRef,
        bitmap_info: u32,
        provider: CFTypeRef,
        decode: *const f64,
        should_interpolate: bool,
        intent: i32,
    ) -> CFTypeRef;
}

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    static kCTFontAttributeName: CFTypeRef;
    static kCTForegroundColorFromContextAttributeName: CFTypeRef;

    fn CTFontCreateCopyWithSymbolicTraits(
        font: CFTypeRef,
        size: f64,
        matrix: *const CGAffineTransform,
        value: u32,
        mask: u32,
    ) -> CFTypeRef;
    fn CTLineCreateWithAttributedString(string: CFTypeRef) -> CFTypeRef;
    fn CTLineDraw(line: CFTypeRef, context: CGContextRef);
    fn CTLineGetTypographicBounds(
        line: CFTypeRef,
        ascent: *mut f64,
        descent: *mut f64,
        leading: *mut f64,
    ) -> f64;
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    static NSFontWeightUltraLight: f64;
    static NSFontWeightThin: f64;
    static NSFontWeightLight: f64;
    static NSFontWeightMedium: f64;
    static NSFontWeightSemibold: f64;
    static NSFontWeightBold: f64;
    static NSFontWeightHeavy: f64;
    static NSFontWeightBlack: f64;
}

const BLEND_MODE_COPY: i32 = 17;
const GRADIENT_EXTENDS_BOTH: u32 = 0b11;
const IMAGE_ALPHA_LAST: u32 = 3;
const INTERPOLATION_HIGH: i32 = 3;
const FONT_ITALIC_TRAIT: u32 = 1 << 0;
const FONT_BOLD_TRAIT: u32 = 1 << 1;

/// Returns the sRGB color space, which every color and image here uses.
fn srgb() -> CfObject {
    // SAFETY: The name constant is valid and the create rule returns a +1 reference.
    unsafe { CfObject::from_retained(CGColorSpaceCreateWithName(kCGColorSpaceSRGB)) }
        .expect("the sRGB color space is always available")
}

/// A 2D affine transform in Core Graphics' layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AffineTransform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

impl From<AffineTransform> for CGAffineTransform {
    fn from(t: AffineTransform) -> Self {
        Self {
            a: t.a,
            b: t.b,
            c: t.c,
            d: t.d,
            tx: t.tx,
            ty: t.ty,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// A color at `offset` in `0.0..=1.0` along a gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}

/// The extent of a line of text drawn at a baseline origin.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub width: f64,
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
}

/// A resolved `NSFont`, which is toll-free bridged to `CTFont`.
#[derive(Clone)]
pub struct Font {
    native: Strong,
}

impl Font {
    /// Resolves a font; an unknown `family` falls back to the system font.
    ///
    /// `weight` uses the CSS scale, where 400 is regular and 700 is bold.
    pub fn new(family: Option<&str>, size: f64, weight: u16, italic: bool) -> Self {
        let named = family.and_then(|family| {
            let name = native::nsstring(family);
            // SAFETY: fontWithName:size: accepts a live NSString and returns an autoreleased font or nil.
            unsafe {
                Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("NSFont"), zpd_objc::sel!("fontWithName:size:"), ((name.as_ptr()): Id, (size): f64) => Id))
            }
        });
        let (font, mut traits) = match named {
            Some(font) => (font, if weight >= 600 { FONT_BOLD_TRAIT } else { 0 }),
            None => {
                let weight = Self::system_weight(weight);
                // SAFETY: systemFontOfSize:weight: returns an autoreleased font.
                let font = unsafe {
                    Strong::retain(zpd_objc::msg_send!(zpd_objc::class!("NSFont"), zpd_objc::sel!("systemFontOfSize:weight:"), ((size): f64, (weight): f64) => Id))
                }
                .expect("the system font is always available");
                (font, 0)
            }
        };
        if italic {
            traits |= FONT_ITALIC_TRAIT;
        }
        if traits == 0 {
            return Self { native: font };
        }
        // SAFETY: The font is a live CTFont; the copy function returns a +1 font or null.
        let styled = unsafe {
            Strong::from_retained(
                CTFontCreateCopyWithSymbolicTraits(
                    font.as_ptr(),
                    0.0,
                    std::ptr::null(),
                    traits,
                    traits,
                )
                .cast_mut(),
            )
        };
        Self {
            native: styled.unwrap_or(font),
        }
    }

    /// Returns the `NSFont`, which lives as long as `self`.
    pub(crate) fn as_ptr(&self) -> Id {
        self.native.as_ptr()
    }

    fn system_weight(weight: u16) -> f64 {
        // SAFETY: AppKit exports these weights as immutable constants.
        unsafe {
            match weight {
                0..150 => NSFontWeightUltraLight,
                150..250 => NSFontWeightThin,
                250..350 => NSFontWeightLight,
                350..450 => native::NSFontWeightRegular,
                450..550 => NSFontWeightMedium,
                550..650 => NSFontWeightSemibold,
                650..750 => NSFontWeightBold,
                750..850 => NSFontWeightHeavy,
                _ => NSFontWeightBlack,
            }
        }
    }

    /// Lays out `text` as one line whose glyphs use the context's fill color.
    fn line(&self, text: &str) -> CfObject {
        let string = native::nsstring(text);
        // SAFETY: Keys are Core Text constants, values are live objects, and both arrays have two entries.
        unsafe {
            let yes = zpd_objc::msg_send!(zpd_objc::class!("NSNumber"), zpd_objc::sel!("numberWithBool:"), ((true): bool) => Id);
            let keys = [
                kCTFontAttributeName.cast_mut(),
                kCTForegroundColorFromContextAttributeName.cast_mut(),
            ];
            let values = [self.native.as_ptr(), yes];
            let attributes = zpd_objc::msg_send!(zpd_objc::class!("NSDictionary"), zpd_objc::sel!("dictionaryWithObjects:forKeys:count:"), ((values.as_ptr()): *const Id, (keys.as_ptr()): *const Id, (2usize): usize) => Id);
            let allocated = zpd_objc::msg_send!(zpd_objc::class!("NSAttributedString"), zpd_objc::sel!("alloc"), () => Id);
            let attributed = Strong::from_retained(zpd_objc::msg_send!(allocated, zpd_objc::sel!("initWithString:attributes:"), ((string.as_ptr()): Id, (attributes): Id) => Id))
                .expect("NSAttributedString allocation failed");
            CfObject::from_retained(CTLineCreateWithAttributedString(attributed.as_ptr()))
                .expect("Core Text failed to create a line")
        }
    }

    pub fn measure(&self, text: &str) -> TextMetrics {
        let line = self.line(text);
        let (mut ascent, mut descent, mut leading) = (0.0, 0.0, 0.0);
        // SAFETY: The line is live and the out-pointers are valid for writes.
        let width = unsafe {
            CTLineGetTypographicBounds(line.as_ptr(), &mut ascent, &mut descent, &mut leading)
        };
        TextMetrics {
            width,
            ascent,
            descent,
            leading,
        }
    }
}

/// An immutable sRGB `CGImage`.
pub struct Image {
    native: CfObject,
    width: usize,
    height: usize,
}

impl Image {
    /// Copies rows-first RGBA8 pixels with straight alpha; returns `None` for a size mismatch.
    pub fn from_rgba8(width: usize, height: usize, pixels: &[u8]) -> Option<Self> {
        if width == 0 || height == 0 || pixels.len() != width * height * 4 {
            return None;
        }
        let space = srgb();
        // SAFETY: CFDataCreate copies `pixels`; the provider and image retain what they use.
        let native = unsafe {
            let data = CfObject::from_retained(CFDataCreate(
                std::ptr::null(),
                pixels.as_ptr(),
                pixels.len() as isize,
            ))?;
            let provider = CGDataProviderCreateWithCFData(data.as_ptr());
            if provider.is_null() {
                return None;
            }
            let image = CGImageCreate(
                width,
                height,
                8,
                32,
                width * 4,
                space.as_ptr(),
                IMAGE_ALPHA_LAST,
                provider,
                std::ptr::null(),
                true,
                0,
            );
            CGDataProviderRelease(provider);
            CfObject::from_retained(image)?
        };
        Some(Self {
            native,
            width,
            height,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

/// The Core Graphics context of a canvas view while it draws.
pub struct GraphicsContext<'frame> {
    context: CGContextRef,
    space: CfObject,
    _frame: PhantomData<&'frame mut ()>,
}

impl GraphicsContext<'_> {
    /// # Safety
    /// `context` must be a live `CGContext` that stays valid for `'frame`.
    pub(crate) unsafe fn new(context: CGContextRef) -> Self {
        let space = srgb();
        // SAFETY: The caller supplies a live context; colors below use this space.
        unsafe {
            CGContextSetFillColorSpace(context, space.as_ptr());
            CGContextSetStrokeColorSpace(context, space.as_ptr());
            CGContextSetInterpolationQuality(context, INTERPOLATION_HIGH);
        }
        Self {
            context,
            space,
            _frame: PhantomData,
        }
    }

    pub fn save(&mut self) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextSaveGState(self.context) };
    }

    pub fn restore(&mut self) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextRestoreGState(self.context) };
    }

    pub fn concat(&mut self, transform: AffineTransform) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextConcatCTM(self.context, transform.into()) };
    }

    pub fn set_alpha(&mut self, alpha: f64) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextSetAlpha(self.context, alpha) };
    }

    pub fn set_antialias(&mut self, enabled: bool) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextSetShouldAntialias(self.context, enabled) };
    }

    /// Starts a new current path.
    pub fn begin_path(&mut self) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextBeginPath(self.context) };
    }

    pub fn move_to(&mut self, point: Point) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextMoveToPoint(self.context, point.x, point.y) };
    }

    pub fn line_to(&mut self, point: Point) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextAddLineToPoint(self.context, point.x, point.y) };
    }

    pub fn quad_to(&mut self, control: Point, point: Point) {
        // SAFETY: The context is live for this frame.
        unsafe {
            CGContextAddQuadCurveToPoint(self.context, control.x, control.y, point.x, point.y)
        };
    }

    pub fn cubic_to(&mut self, first: Point, second: Point, point: Point) {
        // SAFETY: The context is live for this frame.
        unsafe {
            CGContextAddCurveToPoint(
                self.context,
                first.x,
                first.y,
                second.x,
                second.y,
                point.x,
                point.y,
            )
        };
    }

    pub fn close_path(&mut self) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextClosePath(self.context) };
    }

    /// Fills and consumes the current path.
    pub fn fill_path(&mut self, rule: FillRule) {
        // SAFETY: The context is live for this frame.
        unsafe {
            match rule {
                FillRule::NonZero => CGContextFillPath(self.context),
                FillRule::EvenOdd => CGContextEOFillPath(self.context),
            }
        }
    }

    /// Strokes and consumes the current path.
    pub fn stroke_path(&mut self) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextStrokePath(self.context) };
    }

    /// Intersects the clip with the current path and consumes it.
    pub fn clip(&mut self, rule: FillRule) {
        // SAFETY: The context is live for this frame.
        unsafe {
            match rule {
                FillRule::NonZero => CGContextClip(self.context),
                FillRule::EvenOdd => CGContextEOClip(self.context),
            }
        }
    }

    /// Replaces the current path with the outline its stroke would cover.
    pub fn replace_path_with_stroked_path(&mut self) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextReplacePathWithStrokedPath(self.context) };
    }

    pub fn set_fill_color(&mut self, color: Color) {
        let components = [color.red, color.green, color.blue, color.alpha];
        // SAFETY: The fill color space is sRGB, which takes four components.
        unsafe { CGContextSetFillColor(self.context, components.as_ptr()) };
    }

    pub fn set_stroke_color(&mut self, color: Color) {
        let components = [color.red, color.green, color.blue, color.alpha];
        // SAFETY: The stroke color space is sRGB, which takes four components.
        unsafe { CGContextSetStrokeColor(self.context, components.as_ptr()) };
    }

    pub fn set_line_width(&mut self, width: f64) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextSetLineWidth(self.context, width) };
    }

    pub fn set_line_cap(&mut self, cap: LineCap) {
        let cap = match cap {
            LineCap::Butt => 0,
            LineCap::Round => 1,
            LineCap::Square => 2,
        };
        // SAFETY: The value is a CGLineCap.
        unsafe { CGContextSetLineCap(self.context, cap) };
    }

    pub fn set_line_join(&mut self, join: LineJoin) {
        let join = match join {
            LineJoin::Miter => 0,
            LineJoin::Round => 1,
            LineJoin::Bevel => 2,
        };
        // SAFETY: The value is a CGLineJoin.
        unsafe { CGContextSetLineJoin(self.context, join) };
    }

    pub fn set_miter_limit(&mut self, limit: f64) {
        // SAFETY: The context is live for this frame.
        unsafe { CGContextSetMiterLimit(self.context, limit) };
    }

    /// Sets alternating dash and gap lengths; an empty slice draws solid lines.
    pub fn set_line_dash(&mut self, phase: f64, lengths: &[f64]) {
        // SAFETY: The pointer and count describe `lengths`, which Core Graphics copies.
        unsafe { CGContextSetLineDash(self.context, phase, lengths.as_ptr(), lengths.len()) };
    }

    /// Replaces the pixels of `rect`, including alpha, with `color`.
    pub fn clear_rect(&mut self, rect: Rect, color: Color) {
        self.save();
        self.set_fill_color(color);
        // SAFETY: Copy blending writes the color without compositing.
        unsafe {
            CGContextSetBlendMode(self.context, BLEND_MODE_COPY);
            CGContextFillRect(self.context, rect.into());
        }
        self.restore();
    }

    fn gradient(&self, stops: &[GradientStop]) -> Option<CfObject> {
        let components = stops
            .iter()
            .flat_map(|stop| {
                [
                    stop.color.red,
                    stop.color.green,
                    stop.color.blue,
                    stop.color.alpha,
                ]
            })
            .collect::<Vec<_>>();
        let locations = stops.iter().map(|stop| stop.offset).collect::<Vec<_>>();
        // SAFETY: Both arrays hold `stops.len()` sRGB entries; the create rule returns +1.
        unsafe {
            CfObject::from_retained(CGGradientCreateWithColorComponents(
                self.space.as_ptr(),
                components.as_ptr(),
                locations.as_ptr(),
                stops.len(),
            ))
        }
    }

    /// Paints a linear gradient over the current clip, extended past both ends.
    pub fn draw_linear_gradient(&mut self, stops: &[GradientStop], start: Point, end: Point) {
        let Some(gradient) = self.gradient(stops) else {
            return;
        };
        // SAFETY: The context and gradient are live.
        unsafe {
            CGContextDrawLinearGradient(
                self.context,
                gradient.as_ptr(),
                start.into(),
                end.into(),
                GRADIENT_EXTENDS_BOTH,
            )
        };
    }

    /// Paints a radial gradient over the current clip, extended past its edge.
    pub fn draw_radial_gradient(&mut self, stops: &[GradientStop], center: Point, radius: f64) {
        let Some(gradient) = self.gradient(stops) else {
            return;
        };
        // SAFETY: The context and gradient are live.
        unsafe {
            CGContextDrawRadialGradient(
                self.context,
                gradient.as_ptr(),
                center.into(),
                0.0,
                center.into(),
                radius,
                GRADIENT_EXTENDS_BOTH,
            )
        };
    }

    /// Draws one line of text whose baseline starts at `origin`.
    pub fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        let line = font.line(text);
        self.save();
        self.set_fill_color(color);
        // SAFETY: Core Text draws upward, so the flipped space is flipped back around the baseline.
        unsafe {
            CGContextTranslateCTM(self.context, origin.x, origin.y);
            CGContextScaleCTM(self.context, 1.0, -1.0);
            CGContextSetTextMatrix(
                self.context,
                CGAffineTransform {
                    a: 1.0,
                    b: 0.0,
                    c: 0.0,
                    d: 1.0,
                    tx: 0.0,
                    ty: 0.0,
                },
            );
            CGContextSetTextPosition(self.context, 0.0, 0.0);
            CTLineDraw(line.as_ptr(), self.context);
        }
        self.restore();
    }

    /// Draws `image` scaled into `rect`.
    pub fn draw_image(&mut self, image: &Image, rect: Rect) {
        self.save();
        // SAFETY: Images draw with their origin at the bottom, so the flipped space is flipped back.
        unsafe {
            CGContextTranslateCTM(self.context, rect.x, rect.y + rect.height);
            CGContextScaleCTM(self.context, 1.0, -1.0);
            CGContextDrawImage(
                self.context,
                Rect::new(0.0, 0.0, rect.width, rect.height).into(),
                image.native.as_ptr(),
            );
        }
        self.restore();
    }
}
