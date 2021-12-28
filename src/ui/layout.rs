use std::cmp::{max, min};

use crate::{px::Px, shapes::Extent};

use super::{
    tree::Index,
    widgets::{Block, Column, Panel, Row, Visitor, Widget, WidgetTree},
};

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub size: Extent,
}

pub fn compute_layout(
    tree: &WidgetTree,
    root: Index<Widget>,
    area: Extent,
    output: &mut Vec<Option<Layout>>,
) {
    output.resize(tree.len(), None);
    let mut state = State { tree, output };
    state.visit(root, area);
}

struct State<'a> {
    tree: &'a WidgetTree,
    output: &'a mut [Option<Layout>],
}

impl<'a> State<'a> {
    #[inline(always)]
    fn visit(&mut self, index: Index<Widget>, area: Extent) {
        match self.tree.get(index) {
            Widget::Column(column) => self.visit_column(index, column, area),
            Widget::Row(row) => self.visit_row(index, row, area),
            Widget::Panel(panel) => self.visit_panel(index, panel, area),
            Widget::Block(block) => self.visit_block(index, block, area),
        }
    }
}

impl<'a> Visitor<Extent> for State<'a> {
    fn visit_column(&mut self, index: Index<Widget>, column: &Column, area: Extent) {
        let mut advancing_y = Px(0);
        let mut max_child_width = Px(0);
        for child in self.tree.children(index) {
            self.visit(*child, Extent::new(area.width, area.height - advancing_y));

            let child_layout = self.output[child.get()].as_ref().unwrap();
            advancing_y += child_layout.size.height + column.margin;
            if max_child_width < child_layout.size.width {
                max_child_width = child_layout.size.width
            }
        }

        // Compensate for over margin
        if advancing_y > Px(0) {
            advancing_y -= column.margin;
        }

        assert!(advancing_y <= area.height);

        let final_size = Extent::new(max_child_width, advancing_y);

        self.output[index.get()] = Some(Layout { size: final_size });
    }

    fn visit_row(&mut self, index: Index<Widget>, row: &Row, area: Extent) {
        let mut advancing_x = Px(0);
        let mut max_child_height = Px(0);
        for child in self.tree.children(index) {
            self.visit(*child, Extent::new(area.width - advancing_x, area.height));

            let child_layout = self.output[child.get()].as_ref().unwrap();
            advancing_x += child_layout.size.width + row.margin;
            if max_child_height < child_layout.size.height {
                max_child_height = child_layout.size.height
            }
        }

        // Compensate for over margin
        if advancing_x > Px(0) {
            advancing_x -= row.margin;
        }

        assert!(advancing_x <= area.width);

        let final_size = Extent::new(advancing_x, max_child_height);

        self.output[index.get()] = Some(Layout { size: final_size });
    }

    fn visit_block(&mut self, index: Index<Widget>, block: &Block, area: Extent) {
        let width = max(
            block.min_size.width,
            min(block.size.width, min(block.max_size.width, area.width)),
        );

        let height = max(
            block.min_size.height,
            min(block.size.height, min(block.max_size.height, area.height)),
        );

        self.output[index.get()] = Some(Layout {
            size: Extent::new(width, height),
        });
    }

    fn visit_panel(&mut self, index: Index<Widget>, panel: &Panel, area: Extent) {
        let child_width = area.width - 2 * panel.margin;
        let mut advancing_y = panel.margin;
        let mut max_child_width = Px(0);
        for child in self.tree.children(index) {
            self.visit(*child, Extent::new(child_width, area.height - advancing_y));

            let child_layout = self.output[child.get()].as_ref().unwrap();
            advancing_y += child_layout.size.height + panel.margin;
            if max_child_width < child_layout.size.width {
                max_child_width = child_layout.size.width
            }
        }

        let height = max(advancing_y, panel.min_extent.height);
        let width = max(max_child_width + 2 * panel.margin, panel.min_extent.width);

        let final_size = Extent::new(width, height);

        self.output[index.get()] = Some(Layout { size: final_size });
    }
}
