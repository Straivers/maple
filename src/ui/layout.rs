use crate::{
    gfx::Color,
    px::Px,
    shapes::{Extent, Point, Rect},
    ui::SmoothSlider,
};

use super::{
    widget::{Button, State as WidgetState, Widget},
    Context, DrawCommand,
};

pub const UI_COLOR: Color = Color::rgb(100, 100, 100);
pub const HOVER_COLOR: Color = Color::rgb(200, 200, 200);
pub const ACTIVE_COLOR: Color = Color::rgb(100, 100, 255);

/// Implementors of the [`LayoutState`] interface describe the current state
/// of the layout such as advancing position offsets, and computes the actual
/// position of UI elements within the layout.
pub trait LayoutState {
    fn end_child(&mut self, extent: Extent);

    fn widget_extent(&self) -> (Extent, Extent);

    fn position_extent(&mut self, extent: Extent) -> Rect;
}

#[allow(drop_bounds)]
pub trait Layout: Drop {
    fn context(&mut self) -> &mut Context;

    fn state(&mut self) -> &mut dyn LayoutState;

    fn draw(&mut self, command: DrawCommand);

    /// Maximize width, minimize height
    fn widget<S: Copy, T: Widget<S>>(&mut self, name: &str, widget: &T) -> S {
        let state = self.state();
        let (min, max) = state.widget_extent();
        let rect = state.position_extent(widget.compute_size(min, max));
        let state = widget.compute_state(rect, self.context());
        widget.draw(state, rect, |cmd| {
            debug_assert!(
                cmd.in_bounds(rect),
                "widget \"{}\" rendered outside its bounds (bounds: {:?}, command: {:?})",
                name,
                &rect,
                cmd
            );
            self.draw(cmd)
        });
        state
    }

    fn button(&mut self, name: &str) -> WidgetState {
        let widget = Button {
            id: self.context().named_id(name),
            min_size: Extent::new(Px(10), Px(20)),
            max_size: Extent::new(Px::MAX, Px::MAX),
        };

        self.widget(name, &widget)
    }

    fn smooth_slider(&mut self, name: &str, value: &mut f32) {
        let widget = SmoothSlider {
            id: self.context().named_id(name),
            value: *value,
            max_height: Px(20),
            slider_width: Px(5),
        };

        let state = self.widget(name, &widget);
        *value = state.1;
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

    fn widget_extent(&self) -> (Extent, Extent) {
        let min_height = Px(0);
        let max_height = self.max.height - self.advancing_y;

        (
            Extent::new(Px(0), min_height),
            Extent::new(self.max.width, max_height),
        )
    }

    fn position_extent(&mut self, extent: Extent) -> Rect {
        let point = Point::new(self.x, self.advancing_y);
        self.advancing_y += extent.height + self.margin;
        Rect { point, extent }
    }
}

impl<'a, 'b, 'c> Layout for TopToBottom<'a, 'b, 'c> {
    fn context(&mut self) -> &mut Context {
        self.context
    }

    fn state(&mut self) -> &mut dyn LayoutState {
        &mut self.state
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

/// The [`Columns`] layout splits a horizontal area into `n` regions of equal
/// width, and places one widget in each area, up to `n` ui elements total.
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

    fn widget_extent(&self) -> (Extent, Extent) {
        assert!(self.column < self.num_columns, "too many columns");
        (Extent::default(), Extent::new(self.block_width(), Px::MAX))
    }

    fn position_extent(&mut self, extent: Extent) -> Rect {
        let block_center = self.block_start() + (self.block_width() / 2);
        let point = Point {
            x: self.x + block_center - (extent.width / 2),
            y: self.y,
        };

        self.column += 1;
        self.advancing_x += self.margin;
        self.max_widget_height = self.max_widget_height.max(extent.height);

        Rect { point, extent }
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

    fn state(&mut self) -> &mut dyn LayoutState {
        &mut self.state
    }

    fn draw(&mut self, command: DrawCommand) {
        self.command_buffer.push(command);
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
