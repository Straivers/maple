use std::cmp::{max, min};

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
    size: Extent,
}

trait Widget {
    /// Computes the size of the widget, accounting for its childrens'
    /// requirements.
    fn compute_layout(
        &self,
        index: Index<WidgetStorage>,
        area: Extent,
        tree: &WidgetTree,
        output: &mut [Option<Layout>],
    );

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
    Panel(Panel),
    Block(Block),
    Column(Column),
}

impl Widget for WidgetStorage {
    fn compute_layout(
        &self,
        index: Index<WidgetStorage>,
        area: Extent,
        tree: &WidgetTree,
        output: &mut [Option<Layout>],
    ) {
        match self {
            WidgetStorage::Panel(panel) => panel.compute_layout(index, area, tree, output),
            WidgetStorage::Block(block) => block.compute_layout(index, area, tree, output),
            WidgetStorage::Column(column) => column.compute_layout(index, area, tree, output),
        }
    }

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
            WidgetStorage::Panel(panel) => {
                panel.build_draw_command_list(index, tree, layout, area, callback)
            }
            WidgetStorage::Block(block) => {
                block.build_draw_command_list(index, tree, layout, area, callback)
            }
            WidgetStorage::Column(column) => {
                column.build_draw_command_list(index, tree, layout, area, callback)
            }
        }
    }
}

impl std::fmt::Debug for WidgetStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Panel(panel) => panel.fmt(f),
            Self::Column(column) => column.fmt(f),
            Self::Block(block) => block.fmt(f),
        }
    }
}

pub type WidgetTree = Tree<WidgetStorage>;

impl WidgetTree {
    pub fn compute_layout(
        &self,
        root: Index<WidgetStorage>,
        area: Extent,
        layout_buffer: &mut Vec<Option<Layout>>,
    ) {
        layout_buffer.resize(self.len(), None);
        self.get(root)
            .compute_layout(root, area, self, layout_buffer);
    }

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
    fn compute_layout(
        &self,
        index: Index<WidgetStorage>,
        area: Extent,
        _: &WidgetTree,
        output: &mut [Option<Layout>],
    ) {
        let width = max(
            self.min_size.width,
            min(self.size.width, min(self.max_size.width, area.width)),
        );

        let height = max(
            self.min_size.height,
            min(self.size.height, min(self.max_size.height, area.height)),
        );

        output[index.get()] = Some(Layout {
            size: Extent::new(width, height),
        });
    }

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
    fn compute_layout(
        &self,
        index: Index<WidgetStorage>,
        area: Extent,
        tree: &WidgetTree,
        output: &mut [Option<Layout>],
    ) {
        let mut advancing_y = Px(0);
        let mut max_child_width = Px(0);
        for child in tree.children(index) {
            tree.get(*child).compute_layout(
                *child,
                Extent::new(area.width, area.height - advancing_y),
                tree,
                output,
            );

            let child_layout = output[child.get()].as_ref().unwrap();
            advancing_y += child_layout.size.height + self.margin;
            if max_child_width < child_layout.size.width {
                max_child_width = child_layout.size.width
            }
        }

        // Compensate for over margin
        if advancing_y > Px(0) {
            advancing_y -= self.margin;
        }

        assert!(advancing_y <= area.height);

        let final_size = Extent::new(max_child_width, advancing_y);

        output[index.get()] = Some(Layout { size: final_size });
    }

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
    fn compute_layout(
        &self,
        index: Index<WidgetStorage>,
        area: Extent,
        tree: &WidgetTree,
        output: &mut [Option<Layout>],
    ) {
        let child_width = area.width - 2 * self.margin;
        let mut advancing_y = self.margin;
        let mut max_child_width = Px(0);
        for child in tree.children(index) {
            tree.get(*child).compute_layout(
                *child,
                Extent::new(child_width, area.height - advancing_y),
                tree,
                output,
            );

            let child_layout = output[child.get()].as_ref().unwrap();
            advancing_y += child_layout.size.height + self.margin;
            if max_child_width < child_layout.size.width {
                max_child_width = child_layout.size.width
            }
        }

        let height = max(advancing_y, self.min_extent.height);
        let width = max(max_child_width + 2 * self.margin, self.min_extent.width);

        let final_size = Extent::new(width, height);

        output[index.get()] = Some(Layout { size: final_size });
    }

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
