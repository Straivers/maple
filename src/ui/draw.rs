use crate::{
    gfx::Color,
    px::Px,
    shapes::{Extent, Rect},
};

use super::{
    layout::Layout,
    tree::Index,
    widgets::{Block, Column, Panel, Row, Visitor, Widget, WidgetTree},
};

pub enum DrawCommand {
    Rect { rect: Rect, color: Color },
}

pub fn build_draw_commands<F>(
    tree: &WidgetTree,
    root: Index<Widget>,
    layout: &[Option<Layout>],
    area: Extent,
    callback: &mut F,
) where
    F: FnMut(&DrawCommand),
{
    let mut state = State {
        tree,
        layout,
        callback,
    };
    state.visit(root, Rect::from_extent(Px(0), Px(0), area));
}

struct State<'a, F>
where
    F: FnMut(&DrawCommand),
{
    tree: &'a WidgetTree,
    layout: &'a [Option<Layout>],
    callback: &'a mut F,
}

impl<'a, F> State<'a, F>
where
    F: FnMut(&DrawCommand),
{
    #[inline(always)]
    fn visit(&mut self, index: Index<Widget>, area: Rect) {
        match self.tree.get(index) {
            Widget::Column(column) => self.visit_column(index, column, area),
            Widget::Row(row) => self.visit_row(index, row, area),
            Widget::Panel(panel) => self.visit_panel(index, panel, area),
            Widget::Block(block) => self.visit_block(index, block, area),
        }
    }
}

impl<'a, F> Visitor<Rect> for State<'a, F>
where
    F: FnMut(&DrawCommand),
{
    fn visit_column(&mut self, index: Index<Widget>, column: &Column, area: Rect) {
        let mut advancing_y = area.y();

        for child_index in self.tree.children(index) {
            self.visit(
                *child_index,
                Rect::new(
                    area.x(),
                    advancing_y,
                    area.width(),
                    area.height() - advancing_y,
                ),
            );
            advancing_y += self.layout[child_index.get()].as_ref().unwrap().size.height;
            advancing_y += column.margin;
        }
    }

    fn visit_row(&mut self, index: Index<Widget>, row: &Row, area: Rect) {
        let mut advancing_x = area.x();

        for child_index in self.tree.children(index) {
            self.visit(
                *child_index,
                Rect::new(
                    advancing_x,
                    area.y(),
                    area.width() - advancing_x,
                    area.height(),
                ),
            );
            advancing_x += self.layout[child_index.get()].as_ref().unwrap().size.width;
            advancing_x += row.margin;
        }
    }

    fn visit_block(&mut self, index: Index<Widget>, block: &Block, area: Rect) {
        (self.callback)(&DrawCommand::Rect {
            rect: Rect::from_extent(
                area.x(),
                area.y(),
                self.layout[index.get()].as_ref().unwrap().size,
            ),
            color: block.color,
        });
    }

    fn visit_panel(&mut self, index: Index<Widget>, panel: &Panel, area: Rect) {
        (self.callback)(&DrawCommand::Rect {
            rect: Rect::from_extent(
                area.x(),
                area.y(),
                self.layout[index.get()].as_ref().unwrap().size,
            ),
            color: panel.color,
        });

        let mut advancing_y = area.y() + panel.margin;

        for child_index in self.tree.children(index) {
            self.visit(
                *child_index,
                Rect::new(
                    area.x() + panel.margin,
                    advancing_y,
                    area.width() - 2 * panel.margin,
                    area.height() - advancing_y,
                ),
            );
            advancing_y += self.layout[child_index.get()].as_ref().unwrap().size.height;
            advancing_y += panel.margin;
        }
    }
}
