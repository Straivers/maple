use crate::{
    gfx::Color,
    px::Px,
    shapes::{Extent, Point, Rect},
};

use super::{
    widget::{Button, State as WidgetState, Widget},
    Context, DrawCommand,
};

pub const UI_COLOR: Color = Color::rgb(100, 100, 100);
pub const HOVER_COLOR: Color = Color::rgb(200, 200, 200);
pub const ACTIVE_COLOR: Color = Color::rgb(100, 100, 255);

pub trait LayoutState {
    fn end_child(&mut self, extent: Extent);
}

#[allow(drop_bounds)]
pub trait Layout: Drop {
    fn context(&mut self) -> &mut Context;

    fn widget_extent(&self) -> (Extent, Extent);

    fn position_extent(&mut self, extent: Extent) -> Rect;

    fn draw(&mut self, command: DrawCommand);

    /// Maximize width, minimize height
    fn compute_layout(&mut self, compute_size: impl Fn(Extent, Extent) -> Extent) -> Rect {
        let (min, max) = self.widget_extent();
        let size = compute_size(min, max);
        self.position_extent(size)
    }

    fn button(&mut self, name: &str) -> WidgetState {
        let button = Button {
            id: self.context().named_id(name),
            min_size: Extent::new(Px(10), Px(20)),
            max_size: Extent::new(Px::MAX, Px::MAX),
        };

        let rect = self.compute_layout(|min, max| button.compute_size(min, max));
        let state = button.compute_state(rect, self.context());
        button.draw(state, rect, |cmd| self.draw(cmd));
        state
    }
}

pub struct TopToBottom<'a, 'b, 'c> {
    context: &'a mut Context,
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
        context: &'a mut Context,
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
}

impl<'a, 'b, 'c> Layout for TopToBottom<'a, 'b, 'c> {
    fn context(&mut self) -> &mut Context {
        self.context
    }

    fn draw(&mut self, command: DrawCommand) {
        self.command_buffer.push(command);
    }

    fn widget_extent(&self) -> (Extent, Extent) {
        let min_height = Px(0);
        let max_height = self.state.max.height - self.state.advancing_y;

        (
            Extent::new(Px(0), min_height),
            Extent::new(self.state.max.width, max_height),
        )
    }

    fn position_extent(&mut self, extent: Extent) -> Rect {
        let point = Point::new(self.state.x, self.state.advancing_y);
        self.state.advancing_y += extent.height + self.state.margin;
        Rect { point, extent }
    }
}

impl<'a, 'b, 'c> Drop for TopToBottom<'a, 'b, 'c> {
    fn drop(&mut self) {
        self.parent
            .end_child(Extent::new(Px(0), self.state.advancing_y - self.state.y))
    }
}

pub struct Columns<'a, 'b, 'c> {
    context: &'a mut Context,
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
        context: &'a mut Context,
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
        let block_start = self.state.block_start();
        TopToBottom::begin(
            self.context,
            self.command_buffer,
            &mut self.state,
            block_start,
            y,
            Extent::new(block_width, Px::MAX),
            margin,
        )
    }
}

impl LayoutState for ColumnState {
    fn end_child(&mut self, extent: Extent) {
        self.column += 1;
        self.advancing_x += self.margin;
        self.max_widget_height = self.max_widget_height.max(extent.height);
    }
}

impl ColumnState {
    fn block_width(&self) -> Px {
        let margins = (self.num_columns - 1) * self.margin;
        (self.max.width - margins) / self.num_columns
    }

    fn block_start(&self) -> Px {
        self.x + (self.block_width() + self.margin) * self.column
    }
}

impl<'a, 'b, 'c> Layout for Columns<'a, 'b, 'c> {
    fn context(&mut self) -> &mut Context {
        self.context
    }

    fn draw(&mut self, command: DrawCommand) {
        self.command_buffer.push(command);
    }

    fn widget_extent(&self) -> (Extent, Extent) {
        assert!(
            self.state.column < self.state.num_columns,
            "too many columns"
        );
        (
            Extent::default(),
            Extent::new(self.state.block_width(), Px::MAX),
        )
    }

    fn position_extent(&mut self, extent: Extent) -> Rect {
        let block_center = self.state.block_start() + (self.state.block_width() / 2);
        let point = Point {
            x: self.state.x + block_center - (extent.width / 2),
            y: self.state.y,
        };

        self.state.column += 1;
        self.state.advancing_x += self.state.margin;
        self.state.max_widget_height = self.state.max_widget_height.max(extent.height);

        Rect { point, extent }
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
