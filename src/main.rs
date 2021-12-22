mod array_vec;
mod gfx;
mod px;
mod registry;
mod shapes;
mod sys;
mod traits;
mod ui;

// use clap::App;

use gfx::{Color, RendererWindow};
use px::Px;
use registry::named::{IdOps, StrOps};
use shapes::Extent;
use sys::{ButtonState, Event, EventLoopControl, InputState, MouseButton, PhysicalSize};
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
    let mut registry = registry::named::Registry::new();
    let id = registry
        .insert("color", Color::random_rgb().pack())
        .unwrap();

    spawn_window("Title 1", |input, ui| {
        if input.is_pressed(MouseButton::Left) {
            *registry.get_mut_id(id).unwrap() = Color::random_rgb().pack();
        }

        // create a box
        ui.region(Region::with_children(
            Color::rgb(0, 0, 0),
            Px(10),
            ui::LayoutDirection::LeftToRight,
            &[
                Region::with_children(
                    Color::rgb(200, 200, 200),
                    Px(20),
                    ui::LayoutDirection::LeftToRight,
                    &[Region::new(
                        Color::unpack(*registry.get_id(id).unwrap()),
                        Px(0),
                        ui::LayoutDirection::LeftToRight,
                    )],
                ),
                Region::with_children(
                    Color::rgb(200, 200, 200),
                    Px(10),
                    ui::LayoutDirection::TopToBottom,
                    &[
                        Region::new(
                            Color::rgb(255, 0, 0),
                            Px(0),
                            ui::LayoutDirection::LeftToRight,
                        ),
                        Region::new(
                            Color::rgb(0, 255, 0),
                            Px(0),
                            ui::LayoutDirection::LeftToRight,
                        ),
                        Region::new(
                            Color::rgb(0, 0, 255),
                            Px(0),
                            ui::LayoutDirection::LeftToRight,
                        ),
                    ],
                ),
            ],
        ));
    });

    registry.remove("color").unwrap();
}

pub fn spawn_window(title: &str, mut ui_callback: impl FnMut(&InputState, &mut ui::Builder)) {
    let mut context = RendererWindow::new();
    let mut renderer = gfx::Executor::new();
    let mut window_size = Extent::default();
    let mut input = InputState::new();

    sys::window(title, |control, event| {
        match event {
            Event::Created { size } => {
                window_size = size;
                control.set_min_size(Extent::new(Px(720), Px(480)));
                context.bind(control.handle(), size);
            }
            Event::Destroyed {} => {}
            Event::CloseRequested {} => {
                return EventLoopControl::Stop;
            }
            Event::Input(event) => {
                input.process(event);
            }
            Event::Resized { size } => {
                window_size = size;

                if window_size != Extent::default() {
                    let mut vertices = vec![];
                    let mut indices = vec![];

                    let mut builder = ui::Builder::new();

                    ui_callback(&input, &mut builder);
                    builder.build(window_size, &mut vertices, &mut indices);

                    if let Some(request) = context.draw(window_size, &vertices, &indices) {
                        let _ = renderer.execute(&request);
                    }
                }

                input.reset();
            }
            Event::Update {} => {
                if window_size != Extent::default() {
                    let mut vertices = vec![];
                    let mut indices = vec![];

                    let mut builder = ui::Builder::new();

                    ui_callback(&input, &mut builder);
                    builder.build(window_size, &mut vertices, &mut indices);

                    if let Some(request) = context.draw(window_size, &vertices, &indices) {
                        let _ = renderer.execute(&request);
                    }
                }

                input.reset();
            }
        }
        EventLoopControl::Continue
    });
}
