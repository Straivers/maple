use crate::{gfx::Color, px::Px, shapes::Extent};

use super::{
    widgets::{Block, Column, Panel, WidgetStorage, WidgetTree},
    Index,
};

pub struct WidgetTreeBuilder {
    tree: WidgetTree,
    children_stack: Vec<Index<WidgetStorage>>,
}

impl WidgetTreeBuilder {
    pub fn new() -> Self {
        Self {
            tree: WidgetTree::new(),
            children_stack: vec![],
        }
    }

    pub fn build(mut self) -> (WidgetTree, Index<WidgetStorage>) {
        let index = self
            .tree
            .add(
                &WidgetStorage::Panel(Panel::new(Color::rgb(0, 0, 0), Px(0), None, None)),
                &self.children_stack,
            )
            .unwrap();
        (self.tree, index)
    }

    pub fn layout_columns(&mut self, margin: Px) -> WidgetBuilderScope {
        self.new_child(WidgetStorage::Column(Column { margin }))
    }

    pub fn panel(
        &mut self,
        color: Color,
        margin: Px,
        min_extent: Option<Extent>,
        max_extent: Option<Extent>,
    ) -> WidgetBuilderScope {
        self.new_child(WidgetStorage::Panel(Panel::new(
            color, margin, min_extent, max_extent,
        )))
    }

    fn new_child(&mut self, widget: WidgetStorage) -> WidgetBuilderScope {
        let children_start = self.children_stack.len();
        WidgetBuilderScope {
            widget,
            tree: &mut self.tree,
            children_stack: &mut self.children_stack,
            children_start,
        }
    }
}

pub struct WidgetBuilderScope<'a> {
    widget: WidgetStorage,
    tree: &'a mut WidgetTree,
    children_stack: &'a mut Vec<Index<WidgetStorage>>,
    children_start: usize,
}

impl<'a> WidgetBuilderScope<'a> {
    pub fn block(
        &mut self,
        color: Color,
        size_hint: Extent,
        min_size: Option<Extent>,
        max_size: Option<Extent>,
    ) -> WidgetBuilderScope {
        let min_size = if let Some(min) = min_size {
            min
        } else {
            Extent::default()
        };

        let max_size = if let Some(max) = max_size {
            max
        } else {
            Extent::MAX
        };

        self.new_child(WidgetStorage::Block(Block {
            color,
            size: size_hint,
            min_size,
            max_size,
        }))
    }

    pub fn layout_columns(&mut self, margin: Px) -> WidgetBuilderScope {
        self.new_child(WidgetStorage::Column(Column { margin }))
    }

    pub fn panel(
        &mut self,
        color: Color,
        margin: Px,
        min_extent: Option<Extent>,
        max_extent: Option<Extent>,
    ) -> WidgetBuilderScope {
        self.new_child(WidgetStorage::Panel(Panel::new(
            color, margin, min_extent, max_extent,
        )))
    }

    pub fn panel_fixed(&mut self, color: Color, margin: Px, size: Extent) -> WidgetBuilderScope {
        self.new_child(WidgetStorage::Panel(Panel::fixed_size(color, margin, size)))
    }

    fn new_child(&mut self, widget: WidgetStorage) -> WidgetBuilderScope {
        let children_start = self.children_stack.len();
        WidgetBuilderScope {
            widget,
            tree: &mut self.tree,
            children_stack: &mut self.children_stack,
            children_start,
        }
    }
}

impl<'a> Drop for WidgetBuilderScope<'a> {
    fn drop(&mut self) {
        let index = self
            .tree
            .add(&self.widget, &self.children_stack[self.children_start..])
            .unwrap();

        self.children_stack.truncate(self.children_start);
        self.children_stack.push(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let mut builder = WidgetTreeBuilder::new();

        {
            let mut panel = builder.panel(Color::rgb(100, 0, 0), Px(10), None, None);
            panel.panel(Color::rgb(0, 100, 0), Px(0), None, None);
            panel.panel(Color::rgb(0, 0, 100), Px(0), None, None);
        }

        builder.panel(
            Color::rgb(200, 200, 200),
            Px(100),
            Some(Extent::new(Px(200), Px(400))),
            None,
        );

        let (tree, root) = builder.build();
        // tree.print(root);
    }
}
