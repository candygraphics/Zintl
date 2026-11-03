//! A small, platform-neutral wrapper around Taffy's flexbox layout engine.

use std::error::Error;
use std::fmt;

use std::collections::HashMap;

use taffy::geometry::{Line, Size as TaffySize};
use taffy::style::{
    AlignItems, Dimension as TaffyDimension, Display, FlexDirection, GridPlacement, JustifyContent,
    LengthPercentage, Style as TaffyStyle,
};
use taffy::style_helpers::{auto, fr, line, repeat, span};
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
    Center,
    End,
    #[default]
    Stretch,
}

/// How a container places its children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Arrangement {
    /// Along [`LayoutStyle::axis`].
    #[default]
    Flex,
    /// On top of each other, later children above; the largest child sets the size.
    Overlay,
    /// Row by row into `columns` columns, each as wide as its widest child.
    Grid { columns: u16 },
}

/// A child's cell in a [`Arrangement::Grid`] parent, counted from zero.
///
/// Children without one fill the cells in order, skipping none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridCell {
    pub row: u16,
    pub column: u16,
    /// Columns covered from `column`; `0` covers the rest of the row.
    pub column_span: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutStyle {
    /// The smallest size along each axis; `None` keeps containers at least as
    /// large as their content's minimum, so they do not shrink until it overlaps.
    pub minimum_size: Option<Size>,
    /// The size a leaf's content prefers, such as a label's whole text.
    ///
    /// Rows and grid columns give the leaf this size when there is room, and
    /// shrink it toward `minimum_size` when there is not. Containers ignore it.
    pub content_size: Option<Size>,
    pub axis: Option<Axis>,
    pub gap: f32,
    pub width: LayoutDimension,
    pub height: LayoutDimension,
    /// Share of the parent's remaining main-axis space taken by this node.
    ///
    /// Auto-sized ancestors of a growing node take free space too, so it
    /// has room to grow without marking every stack around it.
    pub grow: f32,
    /// Inset applied to all four edges of a container.
    pub padding: f32,
    pub main_axis_distribution: MainAxisDistribution,
    pub child_sizing: ChildSizing,
    pub cross_axis_alignment: CrossAxisAlignment,
    /// Lays out a window's top-level view under its title bar instead of below it.
    ///
    /// Only top-level views read this; nested views follow their parent.
    pub ignores_safe_area: bool,
    pub arrangement: Arrangement,
    /// This node's cell when its parent is a grid.
    pub grid_cell: Option<GridCell>,
}

impl LayoutStyle {
    pub const fn leaf(minimum_size: Size) -> Self {
        Self {
            minimum_size: Some(minimum_size),
            content_size: None,
            axis: None,
            gap: 0.0,
            width: LayoutDimension::Auto,
            height: LayoutDimension::Auto,
            grow: 0.0,
            padding: 0.0,
            main_axis_distribution: MainAxisDistribution::Start,
            child_sizing: ChildSizing::Intrinsic,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            ignores_safe_area: false,
            arrangement: Arrangement::Flex,
            grid_cell: None,
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
            minimum_size: None,
            content_size: None,
            axis: Some(axis),
            gap,
            width: LayoutDimension::Auto,
            height: LayoutDimension::Auto,
            grow: 0.0,
            padding: 0.0,
            main_axis_distribution: MainAxisDistribution::Start,
            child_sizing: ChildSizing::Intrinsic,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            ignores_safe_area: false,
            arrangement: Arrangement::Flex,
            grid_cell: None,
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
    /// Leaves with a content size carry it as their context for measuring.
    tree: TaffyTree<Size>,
    /// The axes along which each container has a child that takes free space.
    expansions: HashMap<TaffyNodeId, Expansion>,
}

/// Axes along which a node wants the free space of its parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Expansion {
    horizontal: bool,
    vertical: bool,
}

impl Expansion {
    fn along(self, axis: Axis) -> bool {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }

    fn set(&mut self, axis: Axis, expands: bool) {
        match axis {
            Axis::Horizontal => self.horizontal |= expands,
            Axis::Vertical => self.vertical |= expands,
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            horizontal: self.horizontal || other.horizontal,
            vertical: self.vertical || other.vertical,
        }
    }
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
            expansions: HashMap::new(),
        }
    }

    pub fn create_node(
        &mut self,
        style: LayoutStyle,
        children: &[NodeId],
    ) -> Result<NodeId, LayoutError> {
        let children: Vec<_> = children.iter().map(|child| child.0).collect();
        let mut expansion = Expansion::default();
        match style.arrangement {
            Arrangement::Flex => {
                let axis = style.axis.unwrap_or(Axis::Horizontal);
                for child in &children {
                    expansion = expansion.union(self.expand_child(*child, axis)?);
                }
            }
            Arrangement::Overlay => {
                for child in &children {
                    expansion = expansion.union(self.expansion(*child));
                    let mut child_style = self.tree.style(*child)?.clone();
                    child_style.grid_row = Self::cell_line(0, 1);
                    child_style.grid_column = Self::cell_line(0, 1);
                    self.tree.set_style(*child, child_style)?;
                }
            }
            Arrangement::Grid { columns } => {
                let columns = columns.max(1);
                let mut next = 0_u16;
                for child in &children {
                    expansion = expansion.union(self.expansion(*child));
                    let mut child_style = self.tree.style(*child)?.clone();
                    // Declared cells come from `to_taffy_style`; the rest follow in order.
                    if child_style.grid_row.start == GridPlacement::Auto {
                        child_style.grid_row = Self::cell_line(next / columns, 1);
                        child_style.grid_column = Self::cell_line(next % columns, 1);
                        next += 1;
                    }
                    self.tree.set_style(*child, child_style)?;
                }
            }
        }
        if style.child_sizing == ChildSizing::Equal {
            for child in &children {
                let mut child_style = self.tree.style(*child)?.clone();
                child_style.flex_grow = 1.0;
                child_style.flex_basis = TaffyDimension::length(0.0);
                self.tree.set_style(*child, child_style)?;
            }
        }
        // A declared size along an axis absorbs the children's demand for space there.
        expansion.horizontal &= style.width == LayoutDimension::Auto;
        expansion.vertical &= style.height == LayoutDimension::Auto;
        let node = match style.content_size {
            Some(content) if children.is_empty() => self
                .tree
                .new_leaf_with_context(to_taffy_style(style), content)?,
            _ => self
                .tree
                .new_with_children(to_taffy_style(style), &children)?,
        };
        self.expansions.insert(node, expansion);
        Ok(NodeId(node))
    }

    fn expansion(&self, node: TaffyNodeId) -> Expansion {
        self.expansions.get(&node).copied().unwrap_or_default()
    }

    /// Lets `child` of a flex container along `axis` take the free space its own children want.
    ///
    /// A container whose child grows grows too, along the same axis, and
    /// stretches across its parent when that is the child's cross axis. Returns
    /// the axes along which the container wants space because of `child`.
    fn expand_child(&mut self, child: TaffyNodeId, axis: Axis) -> Result<Expansion, LayoutError> {
        let cross = match axis {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        };
        let inner = self.expansion(child);
        let mut style = self.tree.style(child)?.clone();
        let size_along = |style: &TaffyStyle, axis: Axis| match axis {
            Axis::Horizontal => style.size.width,
            Axis::Vertical => style.size.height,
        };
        let declared_grow = style.flex_grow > 0.0;
        if inner.along(axis) && !declared_grow && size_along(&style, axis).is_auto() {
            style.flex_grow = 1.0;
        }
        if inner.along(cross) && size_along(&style, cross).is_auto() {
            style.align_self = Some(AlignItems::STRETCH);
        }
        // A declared size holds even when siblings need more room than there is.
        if size_along(&style, axis).into_option().is_some() {
            style.flex_shrink = 0.0;
        }
        self.tree.set_style(child, style)?;
        let mut expansion = Expansion::default();
        expansion.set(axis, declared_grow || inner.along(axis));
        expansion.set(cross, inner.along(cross));
        Ok(expansion)
    }

    /// Returns the grid lines covering `span` tracks from track `index`.
    fn cell_line(index: u16, span_count: u16) -> Line<GridPlacement> {
        Line {
            start: line(index as i16 + 1),
            end: span(span_count),
        }
    }

    pub fn compute(&mut self, root: NodeId, available: Size) -> Result<(), LayoutError> {
        self.tree.compute_layout_with_measure(
            root.0,
            TaffySize {
                width: AvailableSpace::Definite(available.width),
                height: AvailableSpace::Definite(available.height),
            },
            Self::measure,
        )?;
        Ok(())
    }

    /// Sizes a leaf to its content size, keeping any size layout already fixed.
    fn measure(
        known: TaffySize<Option<f32>>,
        _available: TaffySize<AvailableSpace>,
        _node: TaffyNodeId,
        content: Option<&mut Size>,
        _style: &TaffyStyle,
    ) -> TaffySize<f32> {
        let content = content.map_or(Size::default(), |content| *content);
        TaffySize {
            width: known.width.unwrap_or(content.width),
            height: known.height.unwrap_or(content.height),
        }
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
        self.tree.compute_layout_with_measure(
            root.0,
            TaffySize {
                width: space(width),
                height: space(height),
            },
            Self::measure,
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
    let (display, grid_template_columns, grid_template_rows) = match style.arrangement {
        Arrangement::Flex => (Display::Flex, Vec::new(), Vec::new()),
        Arrangement::Overlay => (Display::Grid, vec![fr(1.0_f32)], vec![fr(1.0_f32)]),
        Arrangement::Grid { columns } => (
            Display::Grid,
            vec![repeat(columns.max(1), vec![auto()])],
            Vec::new(),
        ),
    };
    let (grid_row, grid_column) = match style.grid_cell {
        Some(cell) => (
            LayoutTree::cell_line(cell.row, 1),
            Line {
                start: line(cell.column as i16 + 1),
                end: if cell.column_span == 0 {
                    line(-1)
                } else {
                    span(cell.column_span)
                },
            },
        ),
        None => (Line::default(), Line::default()),
    };
    TaffyStyle {
        display,
        grid_template_columns,
        grid_template_rows,
        grid_row,
        grid_column,
        size: TaffySize {
            width: dimension(style.width),
            height: dimension(style.height),
        },
        flex_grow: style.grow,
        padding: taffy::geometry::Rect::length(style.padding),
        min_size: match style.minimum_size {
            Some(size) => TaffySize {
                width: TaffyDimension::length(size.width),
                height: TaffyDimension::length(size.height),
            },
            None => TaffySize::auto(),
        },
        flex_direction: match style.axis.unwrap_or(Axis::Horizontal) {
            Axis::Horizontal => FlexDirection::Row,
            Axis::Vertical => FlexDirection::Column,
        },
        align_items: match style.cross_axis_alignment {
            CrossAxisAlignment::Start => Some(AlignItems::START),
            CrossAxisAlignment::Center => Some(AlignItems::CENTER),
            CrossAxisAlignment::End => Some(AlignItems::END),
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
    fn cross_axis_alignment_places_children() {
        // Verifies center and end alignment keep the child's width and offset it on the cross axis.
        for (alignment, x) in [
            (CrossAxisAlignment::Start, 0.0),
            (CrossAxisAlignment::Center, 30.0),
            (CrossAxisAlignment::End, 60.0),
        ] {
            let mut tree = LayoutTree::new();
            let child = tree
                .create_node(LayoutStyle::leaf(Size::new(40.0, 20.0)), &[])
                .unwrap();
            let mut root_style = LayoutStyle::stack(Axis::Vertical, 0.0);
            root_style.width = LayoutDimension::Points(100.0);
            root_style.cross_axis_alignment = alignment;
            let root = tree.create_node(root_style, &[child]).unwrap();

            tree.compute(root, Size::new(100.0, 100.0)).unwrap();

            assert_eq!(tree.layout(child).unwrap().x, x);
            assert_eq!(tree.layout(child).unwrap().width, 40.0);
        }
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

    #[test]
    fn overlay_stacks_children_in_one_cell_sized_by_the_largest() {
        // Verifies overlay children share the origin and stretch to the largest child's size.
        let mut tree = LayoutTree::new();
        let base = tree
            .create_node(LayoutStyle::leaf(Size::new(120.0, 80.0)), &[])
            .unwrap();
        let badge = tree
            .create_node(LayoutStyle::leaf(Size::new(30.0, 20.0)), &[])
            .unwrap();
        let mut style = LayoutStyle::stack(Axis::Vertical, 0.0);
        style.arrangement = Arrangement::Overlay;
        let root = tree.create_node(style, &[base, badge]).unwrap();

        tree.compute_fitting(root, None, None).unwrap();

        assert_eq!(tree.layout(root).unwrap().width, 120.0);
        assert_eq!(tree.layout(root).unwrap().height, 80.0);
        let badge = tree.layout(badge).unwrap();
        assert_eq!((badge.x, badge.y, badge.width), (0.0, 0.0, 120.0));
    }

    #[test]
    fn grid_columns_fit_their_widest_cell_and_rows_follow_in_order() {
        // Verifies cells fill rows in order and each column takes its widest cell, with declared cells kept.
        let mut tree = LayoutTree::new();
        let cell = |tree: &mut LayoutTree, width| {
            tree.create_node(LayoutStyle::leaf(Size::new(width, 20.0)), &[])
                .unwrap()
        };
        let cells = [
            cell(&mut tree, 40.0),
            cell(&mut tree, 10.0),
            cell(&mut tree, 15.0),
            cell(&mut tree, 60.0),
        ];
        let mut stripe = LayoutStyle::leaf(Size::new(0.0, 0.0));
        stripe.grid_cell = Some(GridCell {
            row: 1,
            column: 0,
            column_span: 0,
        });
        let stripe = tree.create_node(stripe, &[]).unwrap();
        let mut style = LayoutStyle::stack(Axis::Vertical, 0.0);
        style.arrangement = Arrangement::Grid { columns: 2 };
        style.gap = 4.0;
        let mut children = vec![stripe];
        children.extend(cells);
        let root = tree.create_node(style, &children).unwrap();

        tree.compute_fitting(root, None, None).unwrap();

        let frame = |node| tree.layout(node).unwrap();
        assert_eq!((frame(cells[1]).x, frame(cells[1]).y), (44.0, 0.0));
        assert_eq!((frame(cells[2]).x, frame(cells[2]).y), (0.0, 24.0));
        assert_eq!(frame(cells[3]).x, 44.0);
        assert_eq!(frame(root).width, 104.0);
        let stripe = frame(stripe);
        assert_eq!((stripe.x, stripe.y, stripe.width), (0.0, 24.0, 104.0));
    }

    #[test]
    fn stacks_grow_with_their_growing_children() {
        // Verifies a growing child makes its auto-sized ancestors take free space too, along and across axes.
        let mut tree = LayoutTree::new();
        let header = tree
            .create_node(LayoutStyle::leaf(Size::new(80.0, 20.0)), &[])
            .unwrap();
        let table = tree
            .create_node(LayoutStyle::fill(Size::new(0.0, 0.0)), &[])
            .unwrap();
        let column = tree
            .create_node(LayoutStyle::stack(Axis::Vertical, 0.0), &[header, table])
            .unwrap();
        let spacer = {
            let mut style = LayoutStyle::leaf(Size::new(0.0, 0.0));
            style.grow = 1.0;
            tree.create_node(style, &[]).unwrap()
        };
        let toolbar = tree
            .create_node(LayoutStyle::stack(Axis::Horizontal, 0.0), &[spacer])
            .unwrap();
        // Like a window's content root: a definite size with children at the start.
        let mut root_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        root_style.width = LayoutDimension::Points(300.0);
        root_style.height = LayoutDimension::Points(200.0);
        root_style.cross_axis_alignment = CrossAxisAlignment::Start;
        let root = tree.create_node(root_style, &[toolbar, column]).unwrap();

        tree.compute(root, Size::new(300.0, 200.0)).unwrap();

        assert_eq!(tree.layout(column).unwrap().height, 200.0);
        assert_eq!(tree.layout(table).unwrap().height, 180.0);
        assert_eq!(tree.layout(toolbar).unwrap().width, 300.0);
    }

    #[test]
    fn fixed_sizes_stop_growth_from_spreading() {
        // Verifies a stack with a declared height keeps it although a child grows.
        let mut tree = LayoutTree::new();
        let table = tree
            .create_node(LayoutStyle::fill(Size::new(0.0, 0.0)), &[])
            .unwrap();
        let mut column_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        column_style.height = LayoutDimension::Points(50.0);
        let column = tree.create_node(column_style, &[table]).unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        root_style.height = LayoutDimension::Points(200.0);
        let root = tree.create_node(root_style, &[column]).unwrap();

        tree.compute(root, Size::new(300.0, 200.0)).unwrap();

        assert_eq!(tree.layout(column).unwrap().height, 50.0);
        assert_eq!(tree.layout(table).unwrap().height, 50.0);
    }

    #[test]
    fn grid_cell_stacks_keep_their_content_width_when_space_is_short() {
        // Verifies a stack in a grid cell does not shrink below its children, which would overlap the next column.
        let mut tree = LayoutTree::new();
        let first = tree
            .create_node(LayoutStyle::leaf(Size::new(120.0, 22.0)), &[])
            .unwrap();
        let second = tree
            .create_node(LayoutStyle::leaf(Size::new(120.0, 22.0)), &[])
            .unwrap();
        let cell = tree
            .create_node(LayoutStyle::stack(Axis::Horizontal, 4.0), &[first, second])
            .unwrap();
        let after = tree
            .create_node(LayoutStyle::leaf(Size::new(60.0, 20.0)), &[])
            .unwrap();
        let mut grid_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        grid_style.arrangement = Arrangement::Grid { columns: 2 };
        let grid = tree.create_node(grid_style, &[cell, after]).unwrap();
        let mut root_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        root_style.width = LayoutDimension::Points(200.0);
        root_style.cross_axis_alignment = CrossAxisAlignment::Start;
        let root = tree.create_node(root_style, &[grid]).unwrap();

        tree.compute_fitting(root, Some(200.0), None).unwrap();

        assert_eq!(tree.layout(cell).unwrap().width, 244.0);
        assert_eq!(tree.layout(after).unwrap().x, 244.0);
    }

    #[test]
    fn declared_sizes_do_not_shrink_for_wide_siblings() {
        // Verifies a child with a declared width keeps it although its siblings overflow the row.
        let mut tree = LayoutTree::new();
        let mut pane_style = LayoutStyle::stack(Axis::Vertical, 0.0);
        pane_style.width = LayoutDimension::Points(200.0);
        let pane = tree.create_node(pane_style, &[]).unwrap();
        let wide = tree
            .create_node(LayoutStyle::leaf(Size::new(400.0, 20.0)), &[])
            .unwrap();
        let mut row_style = LayoutStyle::stack(Axis::Horizontal, 0.0);
        row_style.width = LayoutDimension::Points(300.0);
        let row = tree.create_node(row_style, &[pane, wide]).unwrap();

        tree.compute(row, Size::new(300.0, 100.0)).unwrap();

        assert_eq!(tree.layout(pane).unwrap().width, 200.0);
    }

    #[test]
    fn explicit_zero_minimum_lets_stacks_shrink() {
        // Verifies a stack with a declared zero minimum may still shrink below its content.
        let mut tree = LayoutTree::new();
        let content = tree
            .create_node(LayoutStyle::leaf(Size::new(300.0, 20.0)), &[])
            .unwrap();
        let mut stack_style = LayoutStyle::stack(Axis::Horizontal, 0.0);
        stack_style.minimum_size = Some(Size::new(0.0, 0.0));
        let stack = tree.create_node(stack_style, &[content]).unwrap();
        let mut row_style = LayoutStyle::stack(Axis::Horizontal, 0.0);
        row_style.width = LayoutDimension::Points(100.0);
        let row = tree.create_node(row_style, &[stack]).unwrap();

        tree.compute(row, Size::new(100.0, 100.0)).unwrap();

        assert_eq!(tree.layout(stack).unwrap().width, 100.0);
    }

    #[test]
    fn content_sized_leaves_take_their_content_and_shrink_when_short() {
        // Verifies a leaf with a content size keeps it when it fits and shrinks toward its minimum when it does not.
        for (row_width, expected) in [(400.0, 200.0), (150.0, 70.0)] {
            let mut tree = LayoutTree::new();
            let mut label_style = LayoutStyle::leaf(Size::new(0.0, 20.0));
            label_style.content_size = Some(Size::new(200.0, 20.0));
            let label = tree.create_node(label_style, &[]).unwrap();
            let button = tree
                .create_node(LayoutStyle::leaf(Size::new(80.0, 20.0)), &[])
                .unwrap();
            let mut row_style = LayoutStyle::stack(Axis::Horizontal, 0.0);
            row_style.width = LayoutDimension::Points(row_width);
            let row = tree.create_node(row_style, &[label, button]).unwrap();

            tree.compute(row, Size::new(row_width, 100.0)).unwrap();

            assert_eq!(
                tree.layout(label).unwrap().width,
                expected,
                "row {row_width}"
            );
            assert_eq!(tree.layout(button).unwrap().width, 80.0, "row {row_width}");
        }
    }
}
