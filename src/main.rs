mod array_vec;
mod gfx;
mod registry;
mod sys;
mod traits;
mod ui;

// use clap::App;

use gfx::{Color, RendererWindow};
use sys::{Event, EventLoopControl, PhysicalSize};
use ui::Region;

// const ENVIRONMENT_VARIABLES_HELP: &str = "ENVIRONMENT VARIABLES:
//     MAPLE_CHECK_VULKAN=<0|1> Toggles use of Vulkan validation layers if they are available. [Default 1 on debug builds]";

#[derive(Debug)]
struct CliOptions {}

pub fn main() {
    // let _ = App::new("maple")
    //     .version("0.1.0")
    //     .version_short("v")
    //     .after_help(ENVIRONMENT_VARIABLES_HELP)
    //     .get_matches();

    let options = CliOptions {};

    run(&options);
}

#[derive(Debug, Clone, Copy)]
pub enum WindowStatus {
    Unknown,
    Created,
    Destroyed,
}

fn run(_cli_options: &CliOptions) {
    spawn_window("Title 1", |ui| {
        // create a box
        ui.region(Region::with_children(
            Color::rgb(0, 0, 0),
            10.0,
            ui::LayoutDirection::LeftToRight,
            &[
                Region::with_children(
                    Color::rgb(200, 200, 200),
                    20.0,
                    ui::LayoutDirection::LeftToRight,
                    &[Region::new(
                        Color::rgb(255, 0, 0),
                        0.0,
                        ui::LayoutDirection::LeftToRight,
                    )],
                ),
                Region::with_children(
                    Color::rgb(200, 200, 200),
                    10.0,
                    ui::LayoutDirection::TopToBottom,
                    &[
                        Region::new(Color::rgb(0, 255, 0), 0.0, ui::LayoutDirection::LeftToRight),
                        Region::new(Color::rgb(0, 0, 255), 0.0, ui::LayoutDirection::LeftToRight),
                    ],
                ),
            ],
        ));
    });
}

pub fn spawn_window(title: &str, mut ui_callback: impl FnMut(&mut ui::Builder)) {
    let mut context = RendererWindow::new();
    let mut renderer = gfx::Executor::new();
    let mut ui = ui::Context::default();
    let mut window_size = PhysicalSize {
        width: 0,
        height: 0,
    };

    sys::window(title, |control, event| {
        match event {
            Event::Created { size } => {
                window_size = size;
                context.bind(control.handle(), size);
            }
            Event::Destroyed {} => {}
            Event::CloseRequested {} => {
                return EventLoopControl::Stop;
            }
            Event::CursorMove { x_pos, y_pos } => {
                ui.update_cursor(
                    f32::from(x_pos),
                    f32::from(window_size.height) - f32::from(y_pos),
                );
            }
            Event::MouseButton {
                button: _,
                state: _,
            } => {
                // ui.input_click(button, state);
            }
            Event::DoubleClick { button: _ } => {
                // ui.input_db_click(button);
            }
            Event::ScrollWheel {
                scroll_x: _,
                scroll_y: _,
            } => {
                // ui.update_scroll(scroll_x, scroll_y);
            }
            Event::Char { codepoint: _ } => {
                // ui.input_codepoint(codepoint);
            }
            Event::Resized { size } => {
                window_size = size;
                ui.update_window_size(f32::from(window_size.width), f32::from(window_size.height));

                if window_size != PhysicalSize::default() {
                    let mut vertices = vec![];
                    let mut indices = vec![];

                    let mut builder = ui::Builder::new(&mut ui);

                    ui_callback(&mut builder);
                    builder.build(&mut vertices, &mut indices);

                    if let Some(request) = context.draw(window_size, &vertices, &indices) {
                        let _ = renderer.execute(&request);
                    }

                    ui.advance_frame();
                }
            }
            Event::Update {} => {
                if window_size != PhysicalSize::default() {
                    let mut vertices = vec![];
                    let mut indices = vec![];

                    let mut builder = ui::Builder::new(&mut ui);

                    ui_callback(&mut builder);
                    builder.build(&mut vertices, &mut indices);

                    if let Some(request) = context.draw(window_size, &vertices, &indices) {
                        let _ = renderer.execute(&request);
                    }

                    ui.advance_frame();
                }
            }
        }
        EventLoopControl::Continue
    });
}
