use std::rc::Rc;

use zintl_ui_layout::{
    Axis, CrossAxisAlignment, LayoutDimension, LayoutStyle, LayoutTree, Rect as LayoutRect, Size,
};
use zpd_appkit::geometry::Rect as NativeRect;

use super::AppError;
use super::native::NativeNode;

/// A view and the layout inputs of its subtree, detached from backend storage.
pub(super) struct LayoutSnapshot {
    pub(super) style: LayoutStyle,
    pub(super) native: Rc<NativeNode>,
    pub(super) children: Vec<LayoutSnapshot>,
    /// Children laid out inside the view's own content area, such as scroll
    /// content or tab pages.
    pub(super) hosted: Vec<LayoutSnapshot>,
}

impl LayoutSnapshot {
    fn build(&self, layout: &mut LayoutTree) -> Result<BuiltLayout<'_>, AppError> {
        let children = self
            .children
            .iter()
            .map(|child| child.build(layout))
            .collect::<Result<Vec<_>, _>>()?;
        let child_nodes = children
            .iter()
            .map(|child| child.layout)
            .collect::<Vec<_>>();
        let layout_id = layout
            .create_node(self.style, &child_nodes)
            .map_err(AppError::Layout)?;
        Ok(BuiltLayout {
            snapshot: self,
            layout: layout_id,
            children,
        })
    }
}

struct BuiltLayout<'snapshot> {
    snapshot: &'snapshot LayoutSnapshot,
    layout: zintl_ui_layout::NodeId,
    children: Vec<BuiltLayout<'snapshot>>,
}

impl BuiltLayout<'_> {
    /// Applies frames; `parent_height` is `None` inside flipped parents.
    fn apply(&self, layout: &LayoutTree, parent_height: Option<f32>) -> Result<(), AppError> {
        let frame = layout.layout(self.layout).map_err(AppError::Layout)?;
        let native = &self.snapshot.native;
        native
            .as_view()
            .set_frame(native_frame(frame, parent_height))
            .map_err(AppError::Ui)?;
        for child in &self.children {
            child.apply(layout, Some(frame.height))?;
        }
        match native.as_ref() {
            NativeNode::ScrollView(scroll) => {
                let (width, height) = scroll.content_size().map_err(AppError::Ui)?;
                let (width, height) = (width as f32, height as f32);
                let size = HostLayout::place(
                    &self.snapshot.hosted,
                    0.0,
                    Some(width),
                    None,
                    Size::new(width, height),
                    None,
                )?;
                scroll
                    .set_document_size(size.width.max(width) as f64, size.height.max(height) as f64)
                    .map_err(AppError::Ui)?;
            }
            NativeNode::TabView(tabs) => {
                let area = tabs.content_rect().map_err(AppError::Ui)?;
                let size = Size::new(area.width as f32, area.height as f32);
                for page in &self.snapshot.hosted {
                    page.apply_as_root(size)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl LayoutSnapshot {
    /// Lays out this view's children inside a view that is sized externally.
    fn apply_as_root(&self, size: Size) -> Result<(), AppError> {
        let mut layout = LayoutTree::new();
        let built = self
            .children
            .iter()
            .map(|child| child.build(&mut layout))
            .collect::<Result<Vec<_>, _>>()?;
        let mut style = self.style;
        style.width = LayoutDimension::Points(size.width);
        style.height = LayoutDimension::Points(size.height);
        let child_nodes = built.iter().map(|child| child.layout).collect::<Vec<_>>();
        let root = layout
            .create_node(style, &child_nodes)
            .map_err(AppError::Layout)?;
        layout.compute(root, size).map_err(AppError::Layout)?;
        for child in &built {
            child.apply(&layout, Some(size.height))?;
        }
        Ok(())
    }
}

/// Lays out top-level views inside a window or popover content view.
#[derive(Default)]
pub(super) struct HostLayout {
    pub(super) children: Vec<LayoutSnapshot>,
    pub(super) padding: f32,
}

impl HostLayout {
    pub(super) fn layout(&self, bounds: NativeRect) -> Result<(), AppError> {
        let size = Size::new(bounds.width as f32, bounds.height as f32);
        Self::place(
            &self.children,
            self.padding,
            Some(size.width),
            Some(size.height),
            size,
            Some(size.height),
        )?;
        Ok(())
    }

    /// Measures the content when neither axis is constrained.
    pub(super) fn fitting_size(&self) -> Result<Size, AppError> {
        let mut layout = LayoutTree::new();
        let (root, _) = Self::build(
            &self.children,
            self.padding,
            &mut layout,
            None,
            None,
            Size::default(),
        )?;
        layout
            .compute_fitting(root, None, None)
            .map_err(AppError::Layout)?;
        let frame = layout.layout(root).map_err(AppError::Layout)?;
        Ok(Size::new(frame.width, frame.height))
    }

    fn build<'snapshot>(
        children: &'snapshot [LayoutSnapshot],
        padding: f32,
        layout: &mut LayoutTree,
        width: Option<f32>,
        height: Option<f32>,
        minimum: Size,
    ) -> Result<(zintl_ui_layout::NodeId, Vec<BuiltLayout<'snapshot>>), AppError> {
        let built = children
            .iter()
            .map(|child| child.build(layout))
            .collect::<Result<Vec<_>, _>>()?;
        let mut style = LayoutStyle::stack(Axis::Vertical, 0.0);
        style.width = width.map_or(LayoutDimension::Auto, LayoutDimension::Points);
        style.height = height.map_or(LayoutDimension::Auto, LayoutDimension::Points);
        style.minimum_size = minimum;
        style.padding = padding;
        style.cross_axis_alignment = CrossAxisAlignment::Start;
        let child_nodes = built.iter().map(|child| child.layout).collect::<Vec<_>>();
        let root = layout
            .create_node(style, &child_nodes)
            .map_err(AppError::Layout)?;
        Ok((root, built))
    }

    /// Lays out `children` and returns the root size; `None` axes fit the content.
    fn place(
        children: &[LayoutSnapshot],
        padding: f32,
        width: Option<f32>,
        height: Option<f32>,
        minimum: Size,
        parent_height: Option<f32>,
    ) -> Result<Size, AppError> {
        let mut layout = LayoutTree::new();
        let (root, built) = Self::build(children, padding, &mut layout, width, height, minimum)?;
        layout
            .compute_fitting(root, width, height)
            .map_err(AppError::Layout)?;
        for child in &built {
            child.apply(&layout, parent_height)?;
        }
        let frame = layout.layout(root).map_err(AppError::Layout)?;
        Ok(Size::new(frame.width, frame.height))
    }
}

/// Converts a top-left layout frame to AppKit coordinates.
///
/// Unflipped parents measure `y` from their bottom edge, so the frame is
/// mirrored within `parent_height`; flipped parents pass `None`.
pub(super) fn native_frame(frame: LayoutRect, parent_height: Option<f32>) -> NativeRect {
    let y = match parent_height {
        Some(parent_height) => parent_height - frame.y - frame.height,
        None => frame.y,
    };
    NativeRect::new(
        frame.x as f64,
        y as f64,
        frame.width as f64,
        frame.height as f64,
    )
}
