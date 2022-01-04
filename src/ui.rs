use std::hash::{Hash, Hasher};

use crate::{
    gfx::Color,
    px::Px,
    shapes::{Extent, Point, Rect},
};

mod widget;
use ahash::AHasher;
pub use widget::*;

mod layout;
pub use layout::*;

#[derive(Debug)]
pub enum DrawCommand {
    ColoredRect { rect: Rect, color: Color },
}

impl DrawCommand {
    pub fn in_bounds(&self, bounds: Rect) -> bool {
        match self {
            DrawCommand::ColoredRect { rect, color: _ } => bounds.contains_rect(*rect),
        }
    }
}

#[derive(PartialEq)]
pub enum ActiveItem {
    Active(u64),
    Available,
    Locked,
}

pub use ActiveItem::*;

impl Default for ActiveItem {
    fn default() -> Self {
        Self::Available
    }
}

#[derive(Default)]
pub struct Context {
    cursor: Point,
    is_lmb_pressed: bool,

    hover_item: u64,
    active_item: ActiveItem,
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

    fn end(&mut self) {
        if self.is_lmb_pressed {
            if self.active_item == ActiveItem::Available {
                self.active_item = ActiveItem::Locked;
            } else {
                // The user has clicked on an item and held the LMB down.
            }
        } else {
            self.active_item = ActiveItem::Available;
        }
    }

    fn named_id(&self, s: &str) -> u64 {
        let mut hasher = AHasher::default();
        s.hash(&mut hasher);
        hasher.finish()
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
    pub fn no_input(self) -> Builder<'a, 'b> {
        self.finalize()
    }

    pub fn move_cursor(self, position: Point) -> Builder<'a, 'b> {
        self.context.cursor = position;
        self.finalize()
    }

    pub fn lmb_pressed(self, pressed: bool) -> Builder<'a, 'b> {
        self.context.is_lmb_pressed = pressed;
        self.finalize()
    }

    fn finalize(self) -> Builder<'a, 'b> {
        Builder::new(self.ui_size, self.context, self.command_buffer)
    }
}

pub struct Builder<'a, 'b> {
    context: &'a mut Context,
    command_buffer: Option<&'b mut Vec<DrawCommand>>,
    state: BuilderLayoutState,
}

impl<'a, 'b> Builder<'a, 'b> {
    fn new(
        ui_size: Extent,
        context: &'a mut Context,
        command_buffer: &'b mut Vec<DrawCommand>,
    ) -> Self {
        Self {
            context,
            command_buffer: Some(command_buffer),
            state: BuilderLayoutState {
                max: ui_size,
                advancing_y: Px(0),
            },
        }
    }

    pub fn top_to_bottom(&mut self, margin: Px) -> TopToBottom {
        let size = self.state.max;
        TopToBottom::begin(
            self.context,
            self.command_buffer.as_mut().unwrap(),
            &mut self.state,
            Px(0),
            Px(0),
            size,
            margin,
        )
    }

    pub fn build(mut self) -> &'b mut Vec<DrawCommand> {
        self.command_buffer.take().unwrap()
    }
}

struct BuilderLayoutState {
    max: Extent,
    advancing_y: Px,
}

impl LayoutState for BuilderLayoutState {
    fn end_child(&mut self, extent: Extent) {
        self.advancing_y += extent.height;
    }

    fn widget_extent(&self) -> (Extent, Extent) {
        assert!(self.advancing_y <= self.max.height);

        let min_height = Px(0);
        let max_height = self.max.height - self.advancing_y;

        (
            Extent::new(Px(0), min_height),
            Extent::new(self.max.width, max_height),
        )
    }

    fn position_extent(&mut self, extent: Extent) -> Rect {
        let point = Point::new(Px(0), self.advancing_y);
        self.advancing_y += extent.height;
        Rect { point, extent }
    }
}

impl<'a, 'b> Layout for Builder<'a, 'b> {
    fn context(&mut self) -> &mut Context {
        self.context
    }

    fn state(&mut self) -> &mut dyn LayoutState {
        &mut self.state
    }

    fn draw(&mut self, command: DrawCommand) {
        self.command_buffer.as_mut().unwrap().push(command);
    }
}

impl<'a, 'b> Drop for Builder<'a, 'b> {
    fn drop(&mut self) {
        self.context.end();
    }
}
