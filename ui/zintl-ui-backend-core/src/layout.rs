//! Platform-neutral layout of rendered view snapshots.

use std::rc::Rc;

use zintl_ui_layout::{
    Axis, CrossAxisAlignment, LayoutDimension, LayoutError, LayoutStyle, LayoutTree,
    Rect as LayoutRect, Size,
};

/// A native view that receives frames computed by [`HostLayout`].
pub trait LayoutNative: Sized {
    type Error: From<LayoutError>;

    /// Applies a top-left `frame`; `parent_height` is `None` inside flipped parents.
    fn apply_frame(&self, frame: LayoutRect, parent_height: Option<f32>)
    -> Result<(), Self::Error>;

    /// Lays out children hosted in this view's own content area, such as
    /// scroll content or tab pages, after its frame was applied.
    fn layout_hosted(
        &self,
        frame: LayoutRect,
        hosted: &[LayoutSnapshot<Self>],
    ) -> Result<(), Self::Error> {
        let _ = (frame, hosted);
        Ok(())
    }
}

/// A view and the layout inputs of its subtree, detached from backend storage.
pub struct LayoutSnapshot<N> {
    pub style: LayoutStyle,
    pub native: Rc<N>,
    pub children: Vec<LayoutSnapshot<N>>,
    /// Children laid out inside the view's own content area.
    pub hosted: Vec<LayoutSnapshot<N>>,
}

impl<N: LayoutNative> LayoutSnapshot<N> {
    fn build(&self, layout: &mut LayoutTree) -> Result<BuiltLayout<'_, N>, N::Error> {
        let children = self
            .children
            .iter()
            .map(|child| child.build(layout))
            .collect::<Result<Vec<_>, _>>()?;
        let child_nodes = children
            .iter()
            .map(|child| child.layout)
            .collect::<Vec<_>>();
        let layout_id = layout.create_node(self.style, &child_nodes)?;
        Ok(BuiltLayout {
            snapshot: self,
            layout: layout_id,
            children,
        })
    }

    /// Lays out this view's children inside a view that is sized externally.
    pub fn apply_as_root(&self, size: Size) -> Result<(), N::Error> {
        Self::apply_children(self.style, &self.children, size)
    }

    /// Lays out `children` of a container with `style` that is sized externally.
    pub fn apply_children(
        style: LayoutStyle,
        children: &[Self],
        size: Size,
    ) -> Result<(), N::Error> {
        let mut layout = LayoutTree::new();
        let built = children
            .iter()
            .map(|child| child.build(&mut layout))
            .collect::<Result<Vec<_>, _>>()?;
        let mut style = style;
        style.width = LayoutDimension::Points(size.width);
        style.height = LayoutDimension::Points(size.height);
        let child_nodes = built.iter().map(|child| child.layout).collect::<Vec<_>>();
        let root = layout.create_node(style, &child_nodes)?;
        layout.compute(root, size)?;
        for child in &built {
            child.apply(&layout, Some(size.height), 0.0)?;
        }
        Ok(())
    }
}

impl<N> Clone for LayoutSnapshot<N> {
    fn clone(&self) -> Self {
        Self {
            style: self.style,
            native: self.native.clone(),
            children: self.children.clone(),
            hosted: self.hosted.clone(),
        }
    }
}

struct BuiltLayout<'snapshot, N> {
    snapshot: &'snapshot LayoutSnapshot<N>,
    layout: zintl_ui_layout::NodeId,
    children: Vec<BuiltLayout<'snapshot, N>>,
}

impl<N: LayoutNative> BuiltLayout<'_, N> {
    /// Applies frames to this subtree, moving this view down by `offset_y`.
    fn apply(
        &self,
        layout: &LayoutTree,
        parent_height: Option<f32>,
        offset_y: f32,
    ) -> Result<(), N::Error> {
        let mut frame = layout.layout(self.layout)?;
        frame.y += offset_y;
        let native = &self.snapshot.native;
        native.apply_frame(frame, parent_height)?;
        for child in &self.children {
            child.apply(layout, Some(frame.height), 0.0)?;
        }
        native.layout_hosted(frame, &self.snapshot.hosted)
    }
}

/// Lays out top-level views inside a window or popover content view.
pub struct HostLayout<N> {
    pub children: Vec<LayoutSnapshot<N>>,
    pub padding: f32,
}

impl<N> Default for HostLayout<N> {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            padding: 0.0,
        }
    }
}

impl<N: LayoutNative> HostLayout<N> {
    /// Lays out the children to fill `size` below a `top_inset` covered by the title bar.
    ///
    /// Children fill the whole size when one of them ignores the safe area.
    pub fn layout(&self, size: Size, top_inset: f32) -> Result<(), N::Error> {
        let inset = if self
            .children
            .iter()
            .any(|child| child.style.ignores_safe_area)
        {
            0.0
        } else {
            top_inset.clamp(0.0, size.height)
        };
        let safe = Size::new(size.width, size.height - inset);
        let mut layout = LayoutTree::new();
        let (root, built) = Self::build(
            &self.children,
            self.padding,
            &mut layout,
            Some(safe.width),
            Some(safe.height),
            safe,
        )?;
        layout.compute_fitting(root, Some(safe.width), Some(safe.height))?;
        for child in &built {
            child.apply(&layout, Some(size.height), inset)?;
        }
        Ok(())
    }

    /// Measures the content when neither axis is constrained.
    pub fn fitting_size(&self) -> Result<Size, N::Error> {
        let mut layout = LayoutTree::new();
        let (root, _) = Self::build(
            &self.children,
            self.padding,
            &mut layout,
            None,
            None,
            Size::default(),
        )?;
        layout.compute_fitting(root, None, None)?;
        let frame = layout.layout(root)?;
        Ok(Size::new(frame.width, frame.height))
    }

    fn build<'snapshot>(
        children: &'snapshot [LayoutSnapshot<N>],
        padding: f32,
        layout: &mut LayoutTree,
        width: Option<f32>,
        height: Option<f32>,
        minimum: Size,
    ) -> Result<(zintl_ui_layout::NodeId, Vec<BuiltLayout<'snapshot, N>>), N::Error> {
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
        let root = layout.create_node(style, &child_nodes)?;
        Ok((root, built))
    }

    /// Lays out `children` and returns the root size; `None` axes fit the content.
    pub fn place(
        children: &[LayoutSnapshot<N>],
        padding: f32,
        width: Option<f32>,
        height: Option<f32>,
        minimum: Size,
        parent_height: Option<f32>,
    ) -> Result<Size, N::Error> {
        let mut layout = LayoutTree::new();
        let (root, built) = Self::build(children, padding, &mut layout, width, height, minimum)?;
        layout.compute_fitting(root, width, height)?;
        for child in &built {
            child.apply(&layout, parent_height, 0.0)?;
        }
        let frame = layout.layout(root)?;
        Ok(Size::new(frame.width, frame.height))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// Records the frame it receives.
    #[derive(Default)]
    struct Probe {
        frame: Cell<Option<LayoutRect>>,
    }

    impl LayoutNative for Probe {
        type Error = LayoutError;

        fn apply_frame(&self, frame: LayoutRect, _: Option<f32>) -> Result<(), LayoutError> {
            self.frame.set(Some(frame));
            Ok(())
        }
    }

    fn host(style: LayoutStyle) -> (HostLayout<Probe>, Rc<Probe>) {
        let probe = Rc::new(Probe::default());
        let snapshot = LayoutSnapshot {
            style,
            native: probe.clone(),
            children: Vec::new(),
            hosted: Vec::new(),
        };
        (
            HostLayout {
                children: vec![snapshot],
                padding: 0.0,
            },
            probe,
        )
    }

    #[test]
    fn content_starts_below_the_title_bar() {
        // Verifies top-level views fill the area below the title bar inset.
        let (host, probe) = host(LayoutStyle::fill(Size::new(0.0, 0.0)));
        host.layout(Size::new(400.0, 300.0), 28.0).unwrap();
        let frame = probe.frame.get().unwrap();
        assert_eq!((frame.y, frame.height), (28.0, 272.0));
    }

    #[test]
    fn ignoring_the_safe_area_fills_the_window() {
        // Verifies a top-level view that ignores the safe area extends under the title bar.
        let mut style = LayoutStyle::fill(Size::new(0.0, 0.0));
        style.ignores_safe_area = true;
        let (host, probe) = host(style);
        host.layout(Size::new(400.0, 300.0), 28.0).unwrap();
        let frame = probe.frame.get().unwrap();
        assert_eq!((frame.y, frame.height), (0.0, 300.0));
    }
}
