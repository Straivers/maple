//! Maple Engine entry point

use sys::{window::EventLoopControl, window_event::WindowEvent};

// shaders need to be built every time they change...
// applications don't always know which shaders they're going to need ahead of time
// shaders should be compiled ahead of time for release
// shaders should be recompiled on command during debug
// maple runner is a debug-only tool right now, can afford runtime compilation

// use std::{
//     collections::HashMap,
//     time::{Duration, Instant},
// };

use clap::{App, Arg};

// use tokio::{sync::{self, mpsc::{Sender, channel}}, task::{JoinHandle, spawn_blocking}};
use std::{sync::mpsc::{Sender, channel}, thread::{JoinHandle, spawn}};

mod window;
mod renderer;

#[derive(Debug)]
struct CliOptions {
    with_vulkan_validation: bool,
}

pub fn main() {
    let matches = App::new("maple")
        .version("0.1.0")
        .version_short("v")
        .arg(
            Arg::with_name("with_vulkan_validation")
                .long_help("Toggles vulkan validation layers. You must have a recent installation of the Vulkan SDK. This is true by default in debug builds.")
                .long("with-vulkan-validation")
                .takes_value(true)
                .possible_values(&["true", "false"]),
        )
        .get_matches();

    let options = CliOptions {
        with_vulkan_validation: {
            if let Some(enable) = matches.value_of("with_vulkan_validation") {
                enable.parse().unwrap()
            } else {
                cfg!(debug_assertions)
            }
        },
    };

    run(&options)
}

#[derive(Debug, Clone, Copy)]
pub enum WindowStatus {
    Unknown,
    Created,
    Destroyed,
}

fn run(_cli_options: &CliOptions) {
    let (send, _receive) = channel::<WindowStatus>();

    let (render_thread, to_renderer) = spawn_renderer();

    spawn_window("Title 1", send.clone(), to_renderer.clone());
    spawn_window("Title 2", send.clone(), to_renderer.clone());

    std::mem::drop(to_renderer);

    render_thread.join().unwrap();
}

pub fn spawn_renderer() -> (JoinHandle<()>, Sender<renderer::RenderMessage>) {
    // channels
    let (to_renderer, from_windows) = channel::<renderer::RenderMessage>();
    let joiner = spawn(move || {
        let renderer = renderer::Renderer::new();

        while let Ok(message) = from_windows.recv() {
            match message {
                renderer::RenderMessage::Empty => { println!("RM"); }
                renderer::RenderMessage::SubmitAndPresent {
                    ack,
                    fence,
                    commands,
                    semaphore,
                    swapchain,
                    image_index,
                    time_to_next_vsync: _
                } => {
                    renderer.submit(&[commands], &[fence]);
                    renderer.present(&[swapchain], &[image_index], &[semaphore]);
                    ack.send(renderer::RenderResponse::FramePresented).unwrap();
                }
            }
        }
    });

    (joiner, to_renderer)
}

pub fn spawn_window(title: &str, _ack_send: Sender<WindowStatus>, to_renderer: Sender<renderer::RenderMessage>) -> JoinHandle<()> {
    // let context = None;
    // let ui = ui_builder::new();
    // let ui_state = ui_state::new();
    // let to_renderer = renderer.channel();
    let title = title.to_owned();
    spawn(|| {
        window::window(title, move |control, event| {
            match event {
                WindowEvent::Created { window: _, size: _ } => {
                    // let (send, recv) = oneshot::channel();
                    // to_renderer.send(RendererMessage::NewWindow{ send });

                    // match recv.recv() {
                    //     RendererMessage::WindowContext { r_context } => {
                    //         context = r_context;
                    //     }
                    //     _ => panic!("Unexpected renderer message!")
                    // }
                    // ack_send.blocking_send(WindowStatus::Created).unwrap();
                }
                WindowEvent::Destroyed { window: _ } => {
                    // to_renderer.blocking_send(RendererMessage::WindowDestroyed{}).unwrap();
                    // ack_send.blocking_send(WindowStatus::Destroyed).unwrap();
                    return EventLoopControl::Stop
                }
                WindowEvent::CloseRequested { window } => {
                    control.destroy(window);
                },
                WindowEvent::Redraw {} => {
                    // let render_request = context.unwrap().make_request(ui.vertices, ui.indices);

                    // let (send, recv) = oneshot::channel();
                    // to_renderer.send(RendererMessage::RenderRequest{ render_request, send });

                    // match recv.recv() {
                    //     RendererMessage::RenderComplete => {}
                    //     _ => panic!("Unexpected renderer message!")
                    // }
                    to_renderer.send(renderer::RenderMessage::Empty).unwrap();
                }
                _ => {}
            }
            EventLoopControl::Continue
        });
    })
}
