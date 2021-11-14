mod array_vec;
mod gfx;
mod registry;
mod sys;
mod traits;
mod ui;

use clap::App;

use gfx::{Color, Extent, Point, Rect, RendererWindow};
use sys::{EventLoopControl, PhysicalSize, WindowEvent};
use ui::Panel;

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
        ui.panel(Panel {
            rect: Rect {
                lower_left_corner: Point::new(100.0, 100.0),
                extent: Extent::new(200.0, 300.0),
            },
            color: Color::rgb(100, 200, 100),
        });

        ui.panel(Panel {
            rect: Rect {
                lower_left_corner: Point::new(500.0, 0.0),
                extent: Extent::new(200.0, 300.0),
            },
            color: Color::rgb(100, 200, 0),
        });
    });
}

pub fn spawn_window(title: &str, mut ui_callback: impl FnMut(&mut ui::Builder)) {
    let mut context = RendererWindow::new();
    let mut renderer = gfx::Executor::new();
    let mut ui = ui::Context::new();
    let mut window_size = PhysicalSize {
        width: 0,
        height: 0,
    };

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
            WindowEvent::CursorMove { x_pos: _, y_pos: _ } => {}
            WindowEvent::MouseButton {
                button: _,
                state: _,
            } => {}
            WindowEvent::DoubleClick { button: _ } => {}
            WindowEvent::ScrollWheel {
                scroll_x: _,
                scroll_y: _,
            } => {}
            WindowEvent::Char { codepoint: _ } => {}
            WindowEvent::Resized { size } => {
                window_size = size;

                if window_size != PhysicalSize::default() {
                    let mut vertices = vec![];
                    let mut indices = vec![];

                    let mut builder = ui::Builder::new(
                        &mut ui,
                        Rect {
                            lower_left_corner: Point::default(),
                            extent: Extent {
                                width: window_size.width as f32,
                                height: window_size.height as f32,
                            },
                        },
                        &mut vertices,
                        &mut indices,
                    );

                    ui_callback(&mut builder);
                    builder.build();

                    if let Some(request) = context.draw(window_size, &vertices, &indices) {
                        let _ = renderer.execute(&request);
                    }
                }
            }
            WindowEvent::Update {} => {
                if window_size != PhysicalSize::default() {
                    let mut vertices = vec![];
                    let mut indices = vec![];

                    let mut builder = ui::Builder::new(
                        &mut ui,
                        Rect {
                            lower_left_corner: Point::default(),
                            extent: Extent {
                                width: window_size.width as f32,
                                height: window_size.height as f32,
                            },
                        },
                        &mut vertices,
                        &mut indices,
                    );

                    ui_callback(&mut builder);
                    builder.build();

                    if let Some(request) = context.draw(window_size, &vertices, &indices) {
                        let _ = renderer.execute(&request);
                    }
                }
            }
        }
        EventLoopControl::Continue
    });
}
