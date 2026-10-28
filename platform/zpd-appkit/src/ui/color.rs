use zpd_objc::Id;

/// An sRGB color with components in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Color {
    pub const fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Returns an autoreleased `NSColor` in the sRGB color space.
    pub(crate) fn native(self) -> Id {
        // SAFETY: The NSColor factory accepts CGFloat components and returns an autoreleased object.
        unsafe {
            zpd_objc::msg_send!(zpd_objc::class!("NSColor"), zpd_objc::sel!("colorWithSRGBRed:green:blue:alpha:"), ((self.red): f64, (self.green): f64, (self.blue): f64, (self.alpha): f64) => Id)
        }
    }

    /// Reads an `NSColor` after converting it to sRGB.
    ///
    /// # Safety
    /// `color` must be nil or a live `NSColor`.
    pub(crate) unsafe fn from_native(color: Id) -> Self {
        if color.is_null() {
            return Self::new(0.0, 0.0, 0.0, 1.0);
        }
        let mut value = Self::new(0.0, 0.0, 0.0, 1.0);
        // SAFETY: The caller supplies a live NSColor; conversion may return nil for patterns.
        unsafe {
            let space = zpd_objc::msg_send!(zpd_objc::class!("NSColorSpace"), zpd_objc::sel!("sRGBColorSpace"), () => Id);
            let converted = zpd_objc::msg_send!(color, zpd_objc::sel!("colorUsingColorSpace:"), ((space): Id) => Id);
            if !converted.is_null() {
                zpd_objc::msg_send!(converted, zpd_objc::sel!("getRed:green:blue:alpha:"), ((&mut value.red): *mut f64, (&mut value.green): *mut f64, (&mut value.blue): *mut f64, (&mut value.alpha): *mut f64) => ());
            }
        }
        value
    }
}
