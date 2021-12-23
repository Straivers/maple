mod array_vec;
mod gfx;
mod px;
mod registry;
mod shapes;
mod sys;
mod traits;
mod ui;

// use clap::App;

use gfx::{Canvas, Color, Draw, RendererWindow};
use px::Px;
use registry::named::{IdOps, StrOps};
use shapes::{Extent, Rect};
use sys::{Event, EventLoopControl, InputState, MouseButton};
use ui::{compute_layout, LayoutTree, Region, WidgetTreeBuilder};

use crate::ui::Index;

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
            let mut panel = widgets.panel(Color::rgb(100, 0, 0), Px(10), None, None);

            {
                let mut panel2 = panel.panel(Color::rgb(200, 200, 200), Px(10), None, None);
                panel2.panel_fixed(Color::rgb(0, 100, 0), Px(0), Extent::new(Px(100), Px(50)));
                panel2.panel_fixed(
                    Color::unpack(*registry.get_id(id).unwrap()),
                    Px(0),
                    Extent::new(Px(200), Px(50)),
                );
                panel2.panel_fixed(Color::rgb(0, 0, 200), Px(0), Extent::new(Px(150), Px(10)));
            }

            panel.panel_fixed(
                Color::rgb(130, 133, 133),
                Px(0),
                Extent::new(Px(400), Px(150)),
            );
        }

        widgets.panel_fixed(
            Color::rgb(200, 200, 250),
            Px(0),
            Extent::new(Px(100), Px(200)),
        );

        let (widget_tree, root_widget) = widgets.build();

        let mut layout = LayoutTree::new();
        let layout_root = compute_layout(
            &widget_tree,
            root_widget,
            Rect::from_extent(Px(0), Px(0), canvas.size()),
            &mut layout,
        );

        fn draw(tree: &LayoutTree, root: Index<Region>, canvas: &mut Canvas) {
            canvas.draw(tree.get(root));
            for child in tree.children(root) {
                draw(tree, *child, canvas);
            }
        }

        draw(&layout, layout_root, canvas);
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
