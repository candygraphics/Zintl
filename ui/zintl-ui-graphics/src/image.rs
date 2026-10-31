use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// An immutable RGBA8 bitmap with straight (non-premultiplied) alpha.
///
/// Clones share their pixels and identity, so backends can cache the native
/// image they create for it.
#[derive(Clone)]
pub struct Image {
    id: u64,
    width: u32,
    height: u32,
    pixels: Arc<[u8]>,
}

impl Image {
    /// Wraps `pixels`, which hold `width * height` rows-first RGBA8 values.
    pub fn from_rgba8(
        width: u32,
        height: u32,
        pixels: impl Into<Arc<[u8]>>,
    ) -> Result<Self, ImageError> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let pixels = pixels.into();
        let expected = width as usize * height as usize * 4;
        if width == 0 || height == 0 || pixels.len() != expected {
            return Err(ImageError {
                width,
                height,
                length: pixels.len(),
            });
        }
        Ok(Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            pixels,
        })
    }

    /// Returns an identity shared by clones of this image.
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for Image {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Image")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// The pixel buffer does not match the declared image size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageError {
    pub width: u32,
    pub height: u32,
    pub length: usize,
}

impl fmt::Display for ImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "a {}x{} RGBA8 image needs {} bytes, got {}",
            self.width,
            self.height,
            self.width as usize * self.height as usize * 4,
            self.length
        )
    }
}

impl std::error::Error for ImageError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_length_must_match_size() {
        // Verifies images reject buffers that are not width * height * 4 bytes.
        assert!(Image::from_rgba8(2, 2, vec![0; 15]).is_err());
        assert!(Image::from_rgba8(2, 2, vec![0; 16]).is_ok());
    }

    #[test]
    fn clones_share_identity() {
        // Verifies backends can cache native images by the shared id.
        let image = Image::from_rgba8(1, 1, vec![0; 4]).unwrap();
        let other = Image::from_rgba8(1, 1, vec![0; 4]).unwrap();
        assert_eq!(image.clone(), image);
        assert_ne!(image, other);
    }
}
