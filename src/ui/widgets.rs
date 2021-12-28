use crate::{gfx::Color, px::Px, shapes::Extent};

use super::tree::{Index, Tree};

#[derive(Clone)]
pub enum Widget {
    Column(Column),
    Row(Row),
    Panel(Panel),
    Block(Block),
}

impl std::fmt::Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Column(column) => column.fmt(f),
            Self::Row(row) => row.fmt(f),
            Self::Panel(panel) => panel.fmt(f),
            Self::Block(block) => block.fmt(f),
        }
    }
}

pub type WidgetTree = Tree<Widget>;

/// The [`Block`] widget is a simple solid-colored rectangle. Useful for
/// blocking out user interfaces or testing new layouts.
///
/// It can be given a strict size, a preferred size, or a preferred size within
/// a size range.
#[derive(Clone, Debug)]
pub struct Block {
    pub color: Color,
    pub max_size: Extent,
    pub min_size: Extent,
    pub size: Extent,
}

/// The [`Column`] widget defines a column-based layout, where each child is
/// placed below the previous one separated by a user-defined margin. The
/// dimensions of the column is the smaller of the area allocated to it by its
/// parent, and the smallest bounds that enclose its children.
#[derive(Clone, Debug)]
pub struct Column {
    pub margin: Px,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub margin: Px,
}

/// A [`Panel`] widget is a flexibly-sized container for child widgets with a
/// `margin`-pixel gap on every side.
///
/// Child widgets are placed vertically from top to bottom, with that same
/// `margin`-pixel gap between each.
#[derive(Clone, Debug)]
pub struct Panel {
    pub color: Color,
    pub margin: Px,
    pub min_extent: Extent,
    pub max_extent: Extent,
}

impl Panel {
    pub fn new(
        color: Color,
        margin: Px,
        min_extent: Option<Extent>,
        max_extent: Option<Extent>,
    ) -> Self {
        Self {
            color,
            margin,
            min_extent: if let Some(min) = min_extent {
                min
            } else {
                Extent::default()
            },
            max_extent: if let Some(max) = max_extent {
                max
            } else {
                Extent::MAX
            },
        }
    }

    pub fn fixed_size(color: Color, margin: Px, size: Extent) -> Self {
        Self {
            color,
            margin,
            min_extent: size,
            max_extent: size,
        }
    }
}

pub trait Visitor<Extra> {
    fn visit_column(&mut self, index: Index<Widget>, column: &Column, extra: Extra);
    fn visit_row(&mut self, index: Index<Widget>, row: &Row, extra: Extra);
    fn visit_block(&mut self, index: Index<Widget>, block: &Block, extra: Extra);
    fn visit_panel(&mut self, index: Index<Widget>, panel: &Panel, extra: Extra);
}
