//! A small, platform-neutral wrapper around Taffy's flexbox layout engine.

use std::error::Error;
use std::fmt;

use taffy::geometry::Size as TaffySize;
use taffy::style::{
    AlignItems, Dimension as TaffyDimension, Display, FlexDirection, JustifyContent,
    LengthPercentage, Style as TaffyStyle,
};
use taffy::{AvailableSpace, NodeId as TaffyNodeId, TaffyTree};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutDimension {
    #[default]
    Auto,
    Points(f32),
    Percent(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainAxisDistribution {
    #[default]
    Start,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChildSizing {
    #[default]
    Intrinsic,
    Equal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossAxisAlignment {
    Start,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutStyle {
    pub minimum_size: Size,
    pub axis: Option<Axis>,
    pub gap: f32,
    pub width: LayoutDimension,
    pub height: LayoutDimension,
    /// Share of the parent's remaining main-axis space taken by this node.
    pub grow: f32,
    /// Inset applied to all four edges of a container.
    pub padding: f32,
    pub main_axis_distribution: MainAxisDistribution,
    pub child_sizing: ChildSizing,
    pub cross_axis_alignment: CrossAxisAlignment,
}

impl LayoutStyle {
    pub const fn leaf(minimum_size: Size) -> Self {
        Self {
            minimum_size,
            axis: None,
            gap: 0.0,
            width: LayoutDimension::Auto,
            height: LayoutDimension::Auto,
            grow: 0.0,
            padding: 0.0,
            main_axis_distribution: MainAxisDistribution::Start,
            child_sizing: ChildSizing::Intrinsic,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
        }
    }

    /// A leaf that fills its parent's width and the remaining main-axis space.
    pub const fn fill(minimum_size: Size) -> Self {
        let mut style = Self::leaf(minimum_size);
        style.width = LayoutDimension::Percent(1.0);
        style.grow = 1.0;
        style
    }

    pub const fn stack(axis: Axis, gap: f32) -> Self {
        Self {
            minimum_size: Size::new(0.0, 0.0),
            axis: Some(axis),
            gap,
            width: LayoutDimension::Auto,
            height: LayoutDimension::Auto,
            grow: 0.0,
            padding: 0.0,
            main_axis_distribution: MainAxisDistribution::Start,
            child_sizing: ChildSizing::Intrinsic,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(TaffyNodeId);

#[derive(Debug)]
pub struct LayoutError(taffy::TaffyError);

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Error for LayoutError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl From<taffy::TaffyError> for LayoutError {
    fn from(error: taffy::TaffyError) -> Self {
        Self(error)
    }
}

pub struct LayoutTree {
    tree: TaffyTree,
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
        }
    }

    pub fn create_node(
        &mut self,
        style: LayoutStyle,
        children: &[NodeId],
    ) -> Result<NodeId, LayoutError> {
        let children: Vec<_> = children.iter().map(|child| child.0).collect();
        if style.child_sizing == ChildSizing::Equal {
            for child in &children {
                let mut child_style = self.tree.style(*child)?.clone();
                child_style.flex_grow = 1.0;
                child_style.flex_basis = TaffyDimension::length(0.0);
                self.tree.set_style(*child, child_style)?;
            }
        }
        Ok(NodeId(
            self.tree
                .new_with_children(to_taffy_style(style), &children)?,
        ))
    }

    pub fn compute(&mut self, root: NodeId, available: Size) -> Result<(), LayoutError> {
        self.tree.compute_layout(
            root.0,
            TaffySize {
                width: AvailableSpace::Definite(available.width),
                height: AvailableSpace::Definite(available.height),
            },
        )?;
        Ok(())
    }

    /// Computes a layout whose unspecified axes fit the content.
    ///
    /// `None` sizes that axis to its max-content extent instead of a definite space.
    pub fn compute_fitting(
        &mut self,
        root: NodeId,
        width: Option<f32>,
        height: Option<f32>,
    ) -> Result<(), LayoutError> {
        let space =
            |value: Option<f32>| value.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite);
        self.tree.compute_layout(
            root.0,
            TaffySize {
                width: space(width),
                height: space(height),
            },
        )?;
        Ok(())
    }

    pub fn layout(&self, node: NodeId) -> Result<Rect, LayoutError> {
        let layout = self.tree.layout(node.0)?;
        Ok(Rect {
            x: layout.location.x,
            y: layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        })
    }
}

fn to_taffy_style(style: LayoutStyle) -> TaffyStyle {
    let gap = LengthPercentage::length(style.gap);
    let dimension = |value: LayoutDimension| match value {
        LayoutDimension::Auto => TaffyDimension::auto(),
        LayoutDimension::Points(points) => TaffyDimension::length(points),
        LayoutDimension::Percent(percent) => TaffyDimension::percent(percent),
    };
    TaffyStyle {
        display: Display::Flex,
        size: TaffySize {
            width: dimension(style.width),
            height: dimension(style.height),
        },
        flex_grow: style.grow,
        padding: taffy::geometry::Rect::length(style.padding),
        min_size: TaffySize {
            width: TaffyDimension::length(style.minimum_size.width),
            height: TaffyDimension::length(style.minimum_size.height),
        },
        flex_direction: match style.axis.unwrap_or(Axis::Horizontal) {
            Axis::Horizontal => FlexDirection::Row,
            Axis::Vertical => FlexDirection::Column,
        },
        align_items: match style.cross_axis_alignment {
            CrossAxisAlignment::Start => Some(AlignItems::START),
            CrossAxisAlignment::Stretch => Some(AlignItems::STRETCH),
        },
        justify_content: match style.main_axis_distribution {
            MainAxisDistribution::Start => None,
            MainAxisDistribution::SpaceBetween => Some(JustifyContent::SPACE_BETWEEN),
        },
        gap: TaffySize {
            width: gap,
            height: gap,
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_stack_applies_minimum_sizes_and_gap() {
        // Verifies the wrapper maps declarative minimum sizes and horizontal gaps to Taffy.
        let mut tree = LayoutTree::new();
        let first = tree
            .create_node(LayoutStyle::leaf(Size::new(80.0, 32.0)), &[])
            .unwrap();
        let second = tree
            .create_node(LayoutStyle::leaf(Size::new(160.0, 28.0)), &[])
            .unwrap();
        let root = tree
            .create_node(LayoutStyle::stack(Axis::Horizontal, 12.0), &[first, second])
            .unwrap();

        tree.compute(root, Size::new(400.0, 100.0)).unwrap();

        assert_eq!(tree.layout(first).unwrap().x, 0.0);
        assert_eq!(tree.layout(first).unwrap().width, 80.0);
        assert_eq!(tree.layout(second).unwrap().x, 92.0);
        assert_eq!(tree.layout(second).unwrap().width, 160.0);
    }

    #[test]
    fn percentage_width_fills_a_definite_parent() {
        // Verifies a percentage width resolves against the parent's declared width.
        let mut tree = LayoutTree::new();
        let mut child_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        child_style.width = LayoutDimension::Percent(1.0);
        let child = tree.create_node(child_style, &[]).unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        root_style.width = LayoutDimension::Points(400.0);
        root_style.cross_axis_alignment = CrossAxisAlignment::Start;
        let root = tree.create_node(root_style, &[child]).unwrap();

        tree.compute(root, Size::new(400.0, 100.0)).unwrap();

        assert_eq!(tree.layout(child).unwrap().width, 400.0);
    }

    #[test]
    fn space_between_distributes_space_beyond_the_minimum_gap() {
        // Verifies space-between keeps child widths and places the outer children at both edges.
        let mut tree = LayoutTree::new();
        let first = tree
            .create_node(LayoutStyle::leaf(Size::new(80.0, 20.0)), &[])
            .unwrap();
        let second = tree
            .create_node(LayoutStyle::leaf(Size::new(160.0, 20.0)), &[])
            .unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Horizontal, 12.0);
        root_style.width = LayoutDimension::Points(400.0);
        root_style.main_axis_distribution = MainAxisDistribution::SpaceBetween;
        let root = tree.create_node(root_style, &[first, second]).unwrap();

        tree.compute(root, Size::new(400.0, 100.0)).unwrap();

        assert_eq!(tree.layout(first).unwrap().x, 0.0);
        assert_eq!(tree.layout(second).unwrap().x, 240.0);
        assert_eq!(tree.layout(second).unwrap().width, 160.0);
    }

    #[test]
    fn equal_children_share_available_width() {
        // Verifies equal sizing divides the width after subtracting the declared gap.
        let mut tree = LayoutTree::new();
        let first = tree
            .create_node(LayoutStyle::leaf(Size::new(80.0, 20.0)), &[])
            .unwrap();
        let second = tree
            .create_node(LayoutStyle::leaf(Size::new(160.0, 20.0)), &[])
            .unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Horizontal, 20.0);
        root_style.width = LayoutDimension::Points(400.0);
        root_style.child_sizing = ChildSizing::Equal;
        let root = tree.create_node(root_style, &[first, second]).unwrap();

        tree.compute(root, Size::new(400.0, 100.0)).unwrap();

        assert_eq!(tree.layout(first).unwrap().width, 190.0);
        assert_eq!(tree.layout(second).unwrap().width, 190.0);
    }

    #[test]
    fn growing_child_fills_remaining_height() {
        // Verifies `grow` lets one child absorb the free main-axis space of a fixed-height column.
        let mut tree = LayoutTree::new();
        let header = tree
            .create_node(LayoutStyle::leaf(Size::new(80.0, 20.0)), &[])
            .unwrap();
        let mut body_style = LayoutStyle::leaf(Size::new(80.0, 40.0));
        body_style.grow = 1.0;
        let body = tree.create_node(body_style, &[]).unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        root_style.height = LayoutDimension::Points(200.0);
        let root = tree.create_node(root_style, &[header, body]).unwrap();

        tree.compute(root, Size::new(100.0, 200.0)).unwrap();

        assert_eq!(tree.layout(body).unwrap().y, 20.0);
        assert_eq!(tree.layout(body).unwrap().height, 180.0);
    }

    #[test]
    fn padding_insets_children() {
        // Verifies container padding offsets children and grows the measured size.
        let mut tree = LayoutTree::new();
        let child = tree
            .create_node(LayoutStyle::leaf(Size::new(40.0, 20.0)), &[])
            .unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        root_style.padding = 12.0;
        let root = tree.create_node(root_style, &[child]).unwrap();

        tree.compute_fitting(root, None, None).unwrap();

        assert_eq!(tree.layout(child).unwrap().x, 12.0);
        assert_eq!(tree.layout(root).unwrap().height, 44.0);
    }

    #[test]
    fn fitting_layout_measures_content_height() {
        // Verifies an unspecified height measures the stacked content instead of the viewport.
        let mut tree = LayoutTree::new();
        let first = tree
            .create_node(LayoutStyle::leaf(Size::new(80.0, 30.0)), &[])
            .unwrap();
        let second = tree
            .create_node(LayoutStyle::leaf(Size::new(80.0, 50.0)), &[])
            .unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Vertical, 10.0);
        root_style.width = LayoutDimension::Points(120.0);
        let root = tree.create_node(root_style, &[first, second]).unwrap();

        tree.compute_fitting(root, Some(120.0), None).unwrap();

        assert_eq!(tree.layout(root).unwrap().height, 90.0);
        assert_eq!(tree.layout(root).unwrap().width, 120.0);
    }

    #[test]
    fn equal_children_preserve_minimum_widths_when_constrained() {
        // Verifies minimum widths take precedence when equal shares would be too small.
        let mut tree = LayoutTree::new();
        let first = tree
            .create_node(LayoutStyle::leaf(Size::new(80.0, 20.0)), &[])
            .unwrap();
        let second = tree
            .create_node(LayoutStyle::leaf(Size::new(160.0, 20.0)), &[])
            .unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Horizontal, 20.0);
        root_style.width = LayoutDimension::Points(200.0);
        root_style.child_sizing = ChildSizing::Equal;
        let root = tree.create_node(root_style, &[first, second]).unwrap();

        tree.compute(root, Size::new(200.0, 100.0)).unwrap();

        assert!(tree.layout(first).unwrap().width >= 80.0);
        assert!(tree.layout(second).unwrap().width >= 160.0);
    }
}
