use crate::{shapes::{Extent, Rect, Point}, px::Px, gfx::Color};

use super::{widget::{Widget, Button}, DrawCommand, Context};


pub trait LayoutState {
    fn end_child(&mut self, extent: Extent);

    /// Maximize width, minimize height
    fn compute_layout(&mut self, widget: &dyn Widget) -> Rect;
}

#[allow(drop_bounds)]
pub trait Layout: Drop {
    /// Maximize width, minimize height
    fn compute_layout(&mut self, widget: &dyn Widget) -> Rect;

    fn draw(&mut self, command: DrawCommand);

    fn button(&mut self, name: &str) -> bool {
        let button = Button {
            id: 0,
            min_size: Extent::new(Px(10), Px(20)),
            max_size: Extent::new(Px::MAX, Px::MAX),
        };

        let rect = self.compute_layout(&button);

        // do hit-testing and whatnot here

        self.draw(DrawCommand::ColoredRect {
            rect,
            color: Color::rgb(200, 200, 200),
        });

        false
    }
}

pub struct TopToBottom<'a, 'b, 'c> {
    context: &'a Context,
    command_buffer: &'b mut Vec<DrawCommand>,
    parent: &'c mut dyn LayoutState,
    state: TopToBottomState,
}

pub struct TopToBottomState {
    x: Px,
    y: Px,
    margin: Px,
    advancing_y: Px,
    max: Extent,
}

impl<'a, 'b, 'c> TopToBottom<'a, 'b, 'c> {
    pub fn begin(
        context: &'a Context,
        command_buffer: &'b mut Vec<DrawCommand>,
        parent: &'c mut dyn LayoutState,
        x: Px,
        y: Px,
        max_size: Extent,
        margin: Px,
    ) -> Self {
        Self {
            context,
            command_buffer,
            parent,
            state: TopToBottomState {
                x,
                y,
                margin,
                advancing_y: y,
                max: max_size,
            },
        }
    }

    pub fn layout_columns(&mut self, num_columns: i16, margin: Px) -> Columns {
        let max = Extent::new(
            self.state.max.width,
            self.state.max.height - self.state.advancing_y,
        );
        let x = self.state.x;
        let y = self.state.advancing_y;
        Columns::begin(
            self.context,
            self.command_buffer,
            &mut self.state,
            x,
            y,
            max,
            margin,
            num_columns,
        )
    }
}

impl LayoutState for TopToBottomState {
    fn end_child(&mut self, extent: Extent) {
        self.advancing_y += extent.height + self.margin;
    }

    fn compute_layout(&mut self, widget: &dyn Widget) -> Rect {
        let min_height = Px(0);
        let max_height = self.max.height - self.advancing_y;

        let extent = widget.compute_size(
            Extent::new(Px(0), min_height),
            Extent::new(self.max.width, max_height),
        );
        let point = Point::new(self.x, self.advancing_y);

        self.advancing_y += extent.height + self.margin;

        Rect { point, extent }
    }
}

impl<'a, 'b, 'c> Layout for TopToBottom<'a, 'b, 'c> {
    fn compute_layout(&mut self, widget: &dyn Widget) -> Rect {
        self.state.compute_layout(widget)
    }

    fn draw(&mut self, command: DrawCommand) {
        self.command_buffer.push(command);
    }
}

impl<'a, 'b, 'c> Drop for TopToBottom<'a, 'b, 'c> {
    fn drop(&mut self) {
        self.parent
            .end_child(Extent::new(Px(0), self.state.advancing_y - self.state.y))
    }
}

pub struct Columns<'a, 'b, 'c> {
    context: &'a Context,
    command_buffer: &'b mut Vec<DrawCommand>,
    parent: &'c mut dyn LayoutState,
    state: ColumnState,
}

struct ColumnState {
    x: Px,
    y: Px,
    margin: Px,
    advancing_x: Px,
    max_widget_height: Px,
    num_columns: i16,
    column: i16,
    max: Extent,
}

impl<'a, 'b, 'c> Columns<'a, 'b, 'c> {
    pub fn begin(
        context: &'a Context,
        command_buffer: &'b mut Vec<DrawCommand>,
        parent: &'c mut dyn LayoutState,
        x: Px,
        y: Px,
        max_size: Extent,
        margin: Px,
        num_columns: i16,
    ) -> Self {
        Self {
            context,
            command_buffer,
            parent,
            state: ColumnState {
                x,
                y,
                margin,
                advancing_x: x,
                max_widget_height: Px(0),
                num_columns,
                column: 0,
                max: max_size,
            },
        }
    }

    pub fn layout_rows(&mut self, margin: Px) -> TopToBottom {
        let y = self.state.y;
        let block_width = self.state.block_width();
        let block_start = self.state.x + self.state.margin + ((block_width + self.state.margin) * self.state.column);
        TopToBottom::begin(
            self.context,
            self.command_buffer,
            &mut self.state,
            block_start,
            y,
            Extent::new(block_width, Px::MAX),
            margin
        )
    }
}

impl LayoutState for ColumnState {
    fn end_child(&mut self, extent: Extent) {
        self.max_widget_height = self.max_widget_height.max(extent.height);
        self.column += 1;
    }

    fn compute_layout(&mut self, widget: &dyn Widget) -> Rect {
        assert!(self.column < self.num_columns, "too many columns");

        self.advancing_x += self.margin;
        assert!(self.advancing_x <= self.max.width);

        let block_width = self.block_width();

        let extent = widget.compute_size(Extent::default(), Extent::new(block_width, Px::MAX));
        let point = Point {
            x: self.x + self.margin + ((block_width + self.margin) * self.column) + (block_width / 2) - (extent.width / 2),
            y: self.y,
        };

        self.max_widget_height = self.max_widget_height.max(extent.height);
        self.column += 1;

        Rect { point, extent }
    }
}

impl ColumnState {
    fn block_width(&self) -> Px {
        let margins = (self.num_columns + 1) * self.margin;
        (self.max.width - margins) / self.num_columns
    }
}

impl<'a, 'b, 'c> Layout for Columns<'a, 'b, 'c> {
    fn draw(&mut self, command: DrawCommand) {
        self.command_buffer.push(command);
    }

    fn compute_layout(&mut self, widget: &dyn Widget) -> Rect {
        self.state.compute_layout(widget)
    }
}

impl<'a, 'b, 'c> Drop for Columns<'a, 'b, 'c> {
    fn drop(&mut self) {
        self.parent.end_child(Extent::new(
            self.state.max.width,
            self.state.max_widget_height,
        ))
    }
}
