mod array_vec;
mod gfx;
mod px;
mod registry;
mod shapes;
mod sys;
mod traits;
mod ui;

// use clap::App;

use gfx::{Canvas, Color, DrawStyled, RendererWindow};
use px::Px;
use registry::named::{IdOps, StrOps};
use shapes::Extent;
use sys::{Event, EventLoopControl, InputState, MouseButton};
use ui::{DrawCommand, WidgetTreeBuilder};

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

    spawn_window("Title 1", |input, canvas| {
        if input.is_pressed(MouseButton::Left) {
            *registry.get_mut_id(id).unwrap() = Color::random_rgb().pack();
        }

        let mut widgets = WidgetTreeBuilder::new();
        {
            let mut column = widgets.layout_columns(Px(0));
            {
                let mut panel = column.panel(Color::rgb(100, 0, 0), Px(10), None, None);

                {
                    let mut column2 = panel.layout_columns(Px(10));
                    column2.panel_fixed(Color::rgb(0, 100, 0), Px(0), Extent::new(Px(100), Px(50)));
                    column2.panel_fixed(
                        Color::unpack(*registry.get_id(id).unwrap()),
                        Px(0),
                        Extent::new(Px(200), Px(50)),
                    );
                    column2.panel_fixed(Color::rgb(0, 0, 200), Px(0), Extent::new(Px(150), Px(10)));
                    column2.block(
                        Color::rgb(0, 0, 0),
                        Extent::new(Px(700), Px(20)),
                        Some(Extent::new(Px(10), Px(10))),
                        None,
                    );
                }
                panel.block(
                    Color::rgb(130, 133, 133),
                    Extent::new(Px(400), Px(150)),
                    None,
                    None,
                );
            }

            {
                let mut row = column.layout_rows(Px(10));
                row.block(
                    Color::rgb(200, 200, 250),
                    Extent::new(Px(100), Px(200)),
                    None,
                    None,
                );
                row.block(
                    Color::rgb(100, 100, 255),
                    Extent::new(Px(200), Px(100)),
                    None,
                    None,
                );
                row.block(
                    Color::rgb(50, 50, 255),
                    Extent::new(Px(200), Px(100)),
                    None,
                    None,
                );
            }
        }

        let (widget_tree, root_widget) = widgets.build();

        let mut layout_buffer = vec![];
        widget_tree.compute_layout(root_widget, canvas.size(), &mut layout_buffer);

        widget_tree.build_draw_command_list(
            root_widget,
            &layout_buffer,
            canvas.size(),
            &mut |command| match command {
                DrawCommand::Rect { rect, color } => canvas.draw_styled(rect, *color),
            },
        )
    });

    registry.remove("color").unwrap();
}

pub fn spawn_window(title: &str, mut ui_callback: impl FnMut(&InputState, &mut Canvas)) {
    let mut context = RendererWindow::new();
    let mut renderer = gfx::Executor::new();
    let mut window_size = Extent::default();
    let mut input = InputState::new();

    sys::window(title, |control, event| {
        match event {
            Event::Created { size } => {
                window_size = size;
                control.set_min_size(Extent::new(Px(100), Px(100)));
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
                    let mut canvas = Canvas::new(window_size);

                    ui_callback(&input, &mut canvas);

                    if let Some(request) =
                        context.draw(window_size, &canvas.vertices(), &canvas.indices())
                    {
                        let _ = renderer.execute(&request);
                    }
                }

                input.reset();
            }
            Event::Update {} => {
                if window_size != Extent::default() {
                    let mut canvas = Canvas::new(window_size);

                    ui_callback(&input, &mut canvas);

                    if let Some(request) =
                        context.draw(window_size, &canvas.vertices(), &canvas.indices())
                    {
                        let _ = renderer.execute(&request);
                    }
                }

                input.reset();
            }
        }
        EventLoopControl::Continue
    });
}
