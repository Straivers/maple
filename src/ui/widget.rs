use crate::{
    px::Px,
    shapes::{Extent, Rect},
};

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

pub trait Widget<T: Copy> {
    fn id(&self) -> u64;

    fn compute_size(&self, min: Extent, max: Extent) -> Extent;

    fn compute_state(&self, rect: Rect, context: &mut Context) -> T;

    fn draw(&self, state: T, rect: Rect, draw: impl FnMut(DrawCommand));
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
        } else if rect.contains_point(context.cursor) {
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

pub struct SmoothSlider {
    pub id: u64,
    pub value: f32,
    pub max_height: Px,
    pub slider_width: Px,
}

impl Widget<(State, f32)> for SmoothSlider {
    fn id(&self) -> u64 {
        self.id
    }

    fn compute_size(&self, min: Extent, max: Extent) -> Extent {
        assert!(self.max_height >= min.height, "widget max size too small");
        let min_size = Extent::new(max.width, self.max_height.min(max.height));
        min_size
    }

    fn compute_state(&self, rect: Rect, context: &mut Context) -> (State, f32) {
        let state = if context.active_item == Active(self.id) {
            State::Active
        } else if rect.contains_point(context.cursor) {
            context.hover_item = self.id;
            if (context.active_item == Available) & context.is_lmb_pressed {
                context.active_item = Active(self.id);
                State::Active
            } else {
                State::Hover
            }
        } else {
            State::Idle
        };

        if state.is_active() {
            let active_area = rect.width() - self.slider_width;
            let proportion =
                ((context.cursor.x - rect.x()).0 as f32 / active_area.0 as f32).clamp(0.0, 1.0);
            (state, proportion)
        } else {
            (state, self.value)
        }
    }

    fn draw(&self, state: (State, f32), rect: Rect, mut draw: impl FnMut(DrawCommand)) {
        let bar_height = (rect.height() / 3).max(Px(1));
        assert!(rect.height() > bar_height);
        let bar_y = (rect.height() - bar_height) / 2 + rect.y();
        draw(DrawCommand::ColoredRect {
            rect: Rect::new(rect.x(), bar_y, rect.width(), bar_height),
            color: UI_COLOR,
        });

        let slider_width = Px(5);
        let active_area = rect.width() - self.slider_width;
        let slider_x = rect.x() + (Px((state.1 * active_area.0 as f32) as i16));
        let slider_color = match state.0 {
            State::Idle => UI_COLOR,
            State::Hover => HOVER_COLOR,
            State::Active => ACTIVE_COLOR,
        };

        draw(DrawCommand::ColoredRect {
            rect: Rect::new(slider_x, rect.y(), slider_width, rect.height()),
            color: slider_color,
        });
    }
}
