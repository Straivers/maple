use crate::{gfx::Color, px::Px, shapes::Extent};

use super::tree::{Index, Tree};

/// A simple widget that is a solid-colored rectangle. Useful for blocking out
/// user interfaces or testing new layouts.
///
/// It can be given a strict size, a preferred size, or a preferred size within
/// a size range.
#[derive(Clone, Debug)]
pub struct Block {
    pub color: Color,
    pub max_size: Extent,
    pub min_size: Extent,
    pub size_hint: Extent,
}

#[derive(Clone, Debug)]
pub struct Column {
    pub margin: Px
}

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

#[derive(Clone)]
pub enum Widget {
    Panel(Panel),
    Block(Block),
}

impl std::fmt::Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Panel(panel) => f
                .debug_struct("Widget::Panel")
                .field("color", &panel.color)
                .field("margin", &panel.margin)
                .field("min_extent", &panel.min_extent)
                .field("max_extent", &panel.max_extent)
                .finish(),
            Self::Block(block) => block.fmt(f)
        }
    }
}

pub type WidgetTree = Tree<Widget>;

pub struct WidgetTreeBuilder {
    tree: WidgetTree,
    children_stack: Vec<Index<Widget>>,
}

impl WidgetTreeBuilder {
    pub fn new() -> Self {
        Self {
            tree: WidgetTree::new(),
            children_stack: vec![],
        }
    }

    pub fn build(mut self) -> (WidgetTree, Index<Widget>) {
        let index = self
            .tree
            .add(
                &Widget::Panel(Panel::new(Color::rgb(0, 0, 0), Px(0), None, None)),
                &self.children_stack,
            )
            .unwrap();
        (self.tree, index)
    }

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

        self.new_child(Widget::Block(Block {color, size_hint, min_size, max_size }))
    }

    pub fn panel(
        &mut self,
        color: Color,
        margin: Px,
        min_extent: Option<Extent>,
        max_extent: Option<Extent>,
    ) -> WidgetBuilderScope {
        self.new_child(Widget::Panel(Panel::new(color, margin, min_extent, max_extent)))
    }

    pub fn panel_fixed(&mut self, color: Color, margin: Px, size: Extent) -> WidgetBuilderScope {
        self.new_child(Widget::Panel(Panel::new(color, margin, Some(size), Some(size))))
    }
    
    fn new_child(&mut self, widget: Widget) -> WidgetBuilderScope {
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
    widget: Widget,
    tree: &'a mut WidgetTree,
    children_stack: &'a mut Vec<Index<Widget>>,
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

        self.new_child(Widget::Block(Block {color, size_hint, min_size, max_size }))
    }

    pub fn panel(
        &mut self,
        color: Color,
        margin: Px,
        min_extent: Option<Extent>,
        max_extent: Option<Extent>,
    ) -> WidgetBuilderScope {
        self.new_child(Widget::Panel(Panel::new(color, margin, min_extent, max_extent)))
    }

    pub fn panel_fixed(&mut self, color: Color, margin: Px, size: Extent) -> WidgetBuilderScope {
        self.new_child(Widget::Panel(Panel::fixed_size(color, margin, size)))
    }

    fn new_child(&mut self, widget: Widget) -> WidgetBuilderScope {
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
        tree.print(root);
    }
}
