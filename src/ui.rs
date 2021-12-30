use crate::{
    gfx::Color,
    shapes::{Point, Rect, Extent},
};

pub enum DrawCommand {
    ColoredRect { rect: Rect, color: Color },
}

#[derive(Default)]
pub struct Context {
    cursor: Point,
    is_lmb_pressed: bool,
}

impl Context {
    pub fn begin<'a, 'b>(
        &'a mut self,
        ui_size: Extent,
        command_buffer: &'b mut Vec<DrawCommand>,
    ) -> InputHandler<'a, 'b> {
        command_buffer.clear();
        InputHandler {
            context: self,
            ui_size,
            command_buffer,
        }
    }
}

/// Type for enforcing 1 input event per rebuild. Could alternatively be done by
/// allowing [`Context`]'s `begin()` function to take an input event. However,
/// that would introduce a dependency upon the [`sys`](crate::sys) module.
pub struct InputHandler<'a, 'b> {
    context: &'a mut Context,
    ui_size: Extent,
    command_buffer: &'b mut Vec<DrawCommand>,
}

impl<'a, 'b> InputHandler<'a, 'b> {
    pub fn move_cursor(self, position: Point) -> Builder<'a, 'b> {
        self.context.cursor = position;
        self.finalize()
    }

    pub fn lmb_pressed(self, pressed: bool) -> Builder<'a, 'b> {
        self.context.is_lmb_pressed = pressed;
        self.finalize()
    }

    fn finalize(self) -> Builder<'a, 'b> {
        Builder {
            context: self.context,
            ui_size: self.ui_size,
            command_buffer: self.command_buffer,
        }
    }
}

pub struct Builder<'a, 'b> {
    context: &'a Context,
    ui_size: Extent,
    command_buffer: &'b mut Vec<DrawCommand>,
}

impl<'a, 'b> Builder<'a, 'b> {
    pub fn button(name: &str) -> bool {
        todo!()
    }

    pub fn build(self) -> &'b mut Vec<DrawCommand> {
        self.command_buffer
    }
}
