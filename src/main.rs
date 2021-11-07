mod array_vec;
mod color;
mod geometry;
mod renderer;
mod sys;

use clap::App;

use renderer::RendererWindow;
use sys::{EventLoopControl, PhysicalSize, WindowEvent};

const ENVIRONMENT_VARIABLES_HELP: &str = "ENVIRONMENT VARIABLES:
    MAPLE_CHECK_VULKAN=<0|1> Toggles use of Vulkan validation layers if they are available. [Default 1 on debug builds]";

#[derive(Debug)]
struct CliOptions {}

pub fn main() {
    let _ = App::new("maple")
        .version("0.1.0")
        .version_short("v")
        .after_help(ENVIRONMENT_VARIABLES_HELP)
        .get_matches();

    let options = CliOptions {};

    run(&options)
}

#[derive(Debug, Clone, Copy)]
pub enum WindowStatus {
    Unknown,
    Created,
    Destroyed,
}

fn run(_cli_options: &CliOptions) {
    spawn_window("Title 1");
}

pub fn spawn_window(title: &str) {
    let mut context = RendererWindow::new();
    let mut renderer = renderer::Executor::new();
    let mut window_size = PhysicalSize { width: 0, height: 0 };

    sys::window(title.to_owned(), |control, event| {
        match event {
            WindowEvent::Created { size } => {
                window_size = size;
                context.bind(control.handle(), size);
            }
            WindowEvent::Destroyed {} => {
                return EventLoopControl::Stop;
            }
            WindowEvent::CloseRequested {} => {
                control.destroy();
            }
            WindowEvent::Resized { size } => {
                window_size = size;

                let vertices = [];
                let indices = [];

                if window_size != PhysicalSize::default() {
                    if let Some(request) = context.draw(window_size, &vertices, &indices) {
                        let _ = renderer.execute(&request);
                    }
                }
            }
            WindowEvent::Update {} => {
                let vertices = [];
                let indices = [];

                if window_size != PhysicalSize::default() {
                    if let Some(request) = context.draw(window_size, &vertices, &indices) {
                        let _ = renderer.execute(&request);
                    }
                }
            }
            WindowEvent::CursorMove { x_pos, y_pos } => {}
            WindowEvent::MouseButton { button, state } => {}
            WindowEvent::ScrollWheel { scroll_x, scroll_y } => {}
            WindowEvent::Char { codepoint } => {
                println!("Char: {}", codepoint);
            }
        }
        EventLoopControl::Continue
    });
}
