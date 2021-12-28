use crate::{
    gfx::Color,
    px::Px,
    shapes::{Extent, Rect},
};

use super::tree::{Index, Tree};

pub enum DrawCommand {
    Rect { rect: Rect, color: Color },
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub size: Extent,
}

trait Widget {
    fn build_draw_command_list<F>(
        &self,
        index: Index<WidgetStorage>,
        tree: &WidgetTree,
        layout: &[Option<Layout>],
        area: Rect,
        callback: &mut F,
    ) where
        F: FnMut(&DrawCommand);
}

#[derive(Clone)]
pub enum WidgetStorage {
    Column(Column),
    Row(Row),
    Panel(Panel),
    Block(Block),
}

impl Widget for WidgetStorage {
    fn build_draw_command_list<F>(
        &self,
        index: Index<WidgetStorage>,
        tree: &WidgetTree,
        layout: &[Option<Layout>],
        area: Rect,
        callback: &mut F,
    ) where
        F: FnMut(&DrawCommand),
    {
        match self {
            WidgetStorage::Column(column) => {
                column.build_draw_command_list(index, tree, layout, area, callback)
            }
            WidgetStorage::Row(row) => {
                row.build_draw_command_list(index, tree, layout, area, callback)
            }
            WidgetStorage::Panel(panel) => {
                panel.build_draw_command_list(index, tree, layout, area, callback)
            }
            WidgetStorage::Block(block) => {
                block.build_draw_command_list(index, tree, layout, area, callback)
            }
        }
    }
}

impl std::fmt::Debug for WidgetStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Column(column) => column.fmt(f),
            Self::Row(row) => row.fmt(f),
            Self::Panel(panel) => panel.fmt(f),
            Self::Block(block) => block.fmt(f),
        }
    }
}

pub type WidgetTree = Tree<WidgetStorage>;

impl WidgetTree {
    pub fn build_draw_command_list<F>(
        &self,
        root: Index<WidgetStorage>,
        layout: &[Option<Layout>],
        area: Extent,
        callback: &mut F,
    ) where
        F: FnMut(&DrawCommand),
    {
        self.get(root).build_draw_command_list(
            root,
            self,
            layout,
            Rect::from_extent(Px(0), Px(0), area),
            callback,
        );
    }
}

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

impl Widget for Block {
    fn build_draw_command_list<F>(
        &self,
        index: Index<WidgetStorage>,
        _: &WidgetTree,
        layout: &[Option<Layout>],
        area: Rect,
        callback: &mut F,
    ) where
        F: FnMut(&DrawCommand),
    {
        callback(&DrawCommand::Rect {
            rect: Rect::from_extent(
                area.x(),
                area.y(),
                layout[index.get()].as_ref().unwrap().size,
            ),
            color: self.color,
        });
    }
}

/// The [`Column`] widget defines a column-based layout, where each child is
/// placed below the previous one separated by a user-defined margin. The
/// dimensions of the column is the smaller of the area allocated to it by its
/// parent, and the smallest bounds that enclose its children.
#[derive(Clone, Debug)]
pub struct Column {
    pub margin: Px,
}

impl Widget for Column {
    fn build_draw_command_list<F>(
        &self,
        index: Index<WidgetStorage>,
        tree: &WidgetTree,
        layout: &[Option<Layout>],
        area: Rect,
        callback: &mut F,
    ) where
        F: FnMut(&DrawCommand),
    {
        let mut advancing_y = area.y();

        for child_index in tree.children(index) {
            let child = tree.get(*child_index);
            child.build_draw_command_list(
                *child_index,
                tree,
                layout,
                Rect::new(
                    area.x(),
                    advancing_y,
                    area.width(),
                    area.height() - advancing_y,
                ),
                callback,
            );
            advancing_y += layout[child_index.get()].as_ref().unwrap().size.height;
            advancing_y += self.margin;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub margin: Px,
}

impl Widget for Row {
    fn build_draw_command_list<F>(
        &self,
        index: Index<WidgetStorage>,
        tree: &WidgetTree,
        layout: &[Option<Layout>],
        area: Rect,
        callback: &mut F,
    ) where
        F: FnMut(&DrawCommand),
    {
        let mut advancing_x = area.x();

        for child_index in tree.children(index) {
            let child = tree.get(*child_index);
            child.build_draw_command_list(
                *child_index,
                tree,
                layout,
                Rect::new(
                    advancing_x,
                    area.y(),
                    area.width() - advancing_x,
                    area.height(),
                ),
                callback,
            );
            advancing_x += layout[child_index.get()].as_ref().unwrap().size.width;
            advancing_x += self.margin;
        }
    }
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

impl Widget for Panel {
    fn build_draw_command_list<F>(
        &self,
        index: Index<WidgetStorage>,
        tree: &WidgetTree,
        layout: &[Option<Layout>],
        area: Rect,
        callback: &mut F,
    ) where
        F: FnMut(&DrawCommand),
    {
        callback(&DrawCommand::Rect {
            rect: Rect::from_extent(
                area.x(),
                area.y(),
                layout[index.get()].as_ref().unwrap().size,
            ),
            color: self.color,
        });

        let mut advancing_y = area.y() + self.margin;

        for child_index in tree.children(index) {
            let child = tree.get(*child_index);
            child.build_draw_command_list(
                *child_index,
                tree,
                layout,
                Rect::new(
                    area.x() + self.margin,
                    advancing_y,
                    area.width() - 2 * self.margin,
                    area.height() - advancing_y,
                ),
                callback,
            );
            advancing_y += layout[child_index.get()].as_ref().unwrap().size.height;
            advancing_y += self.margin;
        }
    }
}

pub trait Visitor<Extra> {
    fn visit_column(&mut self, index: Index<WidgetStorage>, column: &Column, extra: Extra);
    fn visit_row(&mut self, index: Index<WidgetStorage>, row: &Row, extra: Extra);
    fn visit_block(&mut self, index: Index<WidgetStorage>, block: &Block, extra: Extra);
    fn visit_panel(&mut self, index: Index<WidgetStorage>, panel: &Panel, extra: Extra);
}
