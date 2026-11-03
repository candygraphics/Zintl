use zintl_ui_layout::{LayoutDimension, Rect as LayoutRect, Size};

use super::AppError;
use super::native::NativeNode;
use crate::convert::Native;
use zintl_ui_backend_core::layout::{HostLayout, LayoutNative, LayoutSnapshot};
use zpd_appkit::geometry::Rect;

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
            Self::ScrollView(scroll, state) => {
                let (width, height) = scroll.content_size().map_err(AppError::Ui)?;
                let (width, height) = (width as f32, height as f32);
                // Content that scrolls sideways takes its own width, at least the visible one.
                let fixed_width = (!state.horizontal.get()).then_some(width);
                let size = HostLayout::place(
                    hosted,
                    0.0,
                    fixed_width,
                    None,
                    Size::new(width, height),
                    None,
                )?;
                scroll
                    .set_document_size(size.width.max(width) as f64, size.height.max(height) as f64)
                    .map_err(AppError::Ui)?;
                if let Some((x, y)) = state.restore.take() {
                    scroll.scroll_to(x, y).map_err(AppError::Ui)?;
                }
                // Nested views are placed with the document, so the target can be found now.
                if let Some(target) = state.reveal.take() {
                    scroll
                        .reveal(&target.id, target.alignment.native())
                        .map_err(AppError::Ui)?;
                }
            }
            Self::TabView(tabs) => {
                let area = tabs.content_rect().map_err(AppError::Ui)?;
                let size = Size::new(area.width as f32, area.height as f32);
                for page in hosted {
                    page.apply_as_root(size)?;
                }
            }
            Self::List(list, cells) => {
                // Cells stretch across their column; a row is as tall as its tallest cell.
                let columns = list.column_count();
                let widths = (0..columns)
                    .map(|column| list.column_width(column).map(|width| width as f32))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(AppError::Ui)?;
                let layouts = cells.layouts.borrow();
                let mut heights = vec![1.0_f64; hosted.len().div_ceil(columns)];
                for (index, (layout, cell)) in layouts.iter().zip(hosted).enumerate() {
                    let mut cell = cell.clone();
                    cell.style.width = LayoutDimension::Percent(1.0);
                    let mut layout = layout.borrow_mut();
                    *layout = HostLayout {
                        children: vec![cell],
                        padding: 0.0,
                    };
                    let height = HostLayout::place(
                        &layout.children,
                        0.0,
                        Some(widths[index % columns]),
                        None,
                        Size::default(),
                        None,
                    )?
                    .height;
                    let row = &mut heights[index / columns];
                    *row = row.max(height.ceil() as f64);
                }
                list.set_row_heights(&heights).map_err(AppError::Ui)?;
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
