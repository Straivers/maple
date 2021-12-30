mod array_vec;
mod gfx;
mod px;
mod registry;
mod shapes;
mod sys;
mod traits;
mod ui;

// use clap::App;

use gfx::{Canvas, DrawStyled, RendererWindow};
use px::Px;
use shapes::Extent;
use sys::{ButtonState, EventLoopControl, InputEvent, MouseButton, WindowEvent};

#[derive(Debug)]
struct CliOptions {}

pub fn main() {
    run();
}

#[derive(Debug, Clone, Copy)]
pub enum WindowStatus {
    Unknown,
    Created,
    Destroyed,
}

fn run() {
    let mut ui_context = ui::Context::default();
    let mut ui_command_buffer = vec![];

    spawn_window("Title 1", |inputs, canvas| {
        for input in inputs {
            let input_handler = ui_context.begin(canvas.size(), &mut ui_command_buffer);

            let mut ui = match input {
                InputEvent::CursorMove { position } => input_handler.move_cursor(*position),
                InputEvent::MouseButton { button, state } => {
                    if *button == MouseButton::Left {
                        input_handler.lmb_pressed(*state == ButtonState::Pressed)
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            // actually build the ui here


            for command in ui.build() {
                match command {
                    ui::DrawCommand::ColoredRect { rect, color } => {
                        canvas.draw_styled(rect, *color)
                    }
                }
            }
        }
    });
}

/// Always calls ui_callback with at least one event. If no inputs were received
/// since the last call, the [`InputEvent::None`](sys::input::Event) event is
/// used.
pub fn spawn_window(title: &str, mut ui_callback: impl FnMut(&[InputEvent], &mut Canvas)) {
    let mut context = RendererWindow::new();
    let mut renderer = gfx::Executor::new();
    let mut inputs = vec![];

    sys::window(title, |control, event| {
        match event {
            WindowEvent::Created { size } => {
                control.set_min_size(Extent::new(Px(100), Px(100)));
                context.bind(control.handle(), size);
            }
            WindowEvent::Destroyed {} => {}
            WindowEvent::CloseRequested {} => {
                return EventLoopControl::Stop;
            }
            WindowEvent::Input(event) => {
                inputs.push(event);
            }
            WindowEvent::Update { size, resized } => {
                if size != Extent::default() {
                    let mut canvas = Canvas::new(size);

                    if inputs.is_empty() {
                        inputs.push(InputEvent::None);
                    }

                    ui_callback(&inputs, &mut canvas);
                    inputs.clear();

                    if let Some(request) = context.draw(size, canvas.vertices(), canvas.indices()) {
                        let _ = renderer.execute(&request);
                    }
                }
            }
        }
        EventLoopControl::Continue
    });
}
