use zintl_ui_layout::{Rect as LayoutRect, Size};

use super::AppError;
use super::native::NativeNode;
use crate::Rect;
use crate::layout::{HostLayout, LayoutNative, LayoutSnapshot};

impl LayoutNative for NativeNode {
    type Error = AppError;

    fn apply_frame(&self, frame: LayoutRect, parent_height: Option<f32>) -> Result<(), AppError> {
        self.as_view()
            .set_frame(native_frame(frame, parent_height))
            .map_err(AppError::Ui)
    }

    fn layout_hosted(
        &self,
        _frame: LayoutRect,
        hosted: &[LayoutSnapshot<Self>],
    ) -> Result<(), AppError> {
        match self {
            Self::ScrollView(scroll, restore) => {
                let (width, height) = scroll.content_size().map_err(AppError::Ui)?;
                let (width, height) = (width as f32, height as f32);
                let size = HostLayout::place(
                    hosted,
                    0.0,
                    Some(width),
                    None,
                    Size::new(width, height),
                    None,
                )?;
                scroll
                    .set_document_size(size.width.max(width) as f64, size.height.max(height) as f64)
                    .map_err(AppError::Ui)?;
                if let Some((x, y)) = restore.take() {
                    scroll.scroll_to(x, y).map_err(AppError::Ui)?;
                }
            }
            Self::TabView(tabs) => {
                let area = tabs.content_rect().map_err(AppError::Ui)?;
                let size = Size::new(area.width as f32, area.height as f32);
                for page in hosted {
                    page.apply_as_root(size)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Converts a top-left layout frame to AppKit coordinates.
///
/// Unflipped parents measure `y` from their bottom edge, so the frame is
/// mirrored within `parent_height`; flipped parents pass `None`.
pub(super) fn native_frame(frame: LayoutRect, parent_height: Option<f32>) -> Rect {
    let y = match parent_height {
        Some(parent_height) => parent_height - frame.y - frame.height,
        None => frame.y,
    };
    Rect::new(
        frame.x as f64,
        y as f64,
        frame.width as f64,
        frame.height as f64,
    )
}
