use crate::shapes::{Extent, Rect};

use super::{Active, Available, Context, DrawCommand, ACTIVE_COLOR, HOVER_COLOR, UI_COLOR};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Idle,
    Hover,
    Active,
}

impl State {
    pub fn is_hover(self) -> bool {
        self == Self::Hover
    }

    pub fn is_active(self) -> bool {
        self == Self::Active
    }
}

pub trait Widget<T> {
    fn id(&self) -> u64;

    fn compute_size(&self, min: Extent, max: Extent) -> Extent;

    fn compute_state(&self, rect: Rect, context: &mut Context) -> T;

    fn draw(&self, state: State, rect: Rect, draw: impl FnMut(DrawCommand));
}

pub struct Button {
    pub id: u64,
    pub min_size: Extent,
    pub max_size: Extent,
}

impl Widget<State> for Button {
    fn id(&self) -> u64 {
        self.id
    }

    /// Minimize height while maximizing width.
    fn compute_size(&self, min: Extent, max: Extent) -> Extent {
        assert!(self.max_size >= min, "widget max size too small");
        let min_size = Extent::new(
            max.width.min(self.max_size.width),
            min.height.max(self.min_size.height),
        );
        assert!(min_size <= max, "widget too big");
        min_size
    }

    fn compute_state(&self, rect: Rect, context: &mut Context) -> State {
        if context.active_item == Active(self.id) {
            State::Active
        } else if rect.contains(context.cursor) {
            context.hover_item = self.id;
            if (context.active_item == Available) & context.is_lmb_pressed {
                context.active_item = Active(self.id);
                State::Active
            } else {
                State::Hover
            }
        } else {
            State::Idle
        }
    }

    fn draw(&self, state: State, rect: Rect, mut draw: impl FnMut(DrawCommand)) {
        let color = match state {
            State::Idle => UI_COLOR,
            State::Hover => HOVER_COLOR,
            State::Active => ACTIVE_COLOR,
        };

        draw(DrawCommand::ColoredRect { rect, color });
    }
}
