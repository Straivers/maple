//! Maple Engine entry point

use sys::{library::Library, window::EventLoopControl, window_event::WindowEvent};

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

use core::panic;
use std::{sync::mpsc::{Sender, channel, sync_channel}, thread::{spawn, JoinHandle}};

mod renderer;
mod window;

const ENVIRONMENT_VARIABLES_HELP: &str = "ENVIRONMENT VARIABLES:
    MAPLE_CHECK_VULKAN=<0|1> Toggles use of Vulkan validation layers if they are available. [Default 1 on debug builds]";

#[derive(Debug)]
struct CliOptions {}

pub fn main() {
    let matches = App::new("maple")
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

fn run(cli_options: &CliOptions) {
    let (send, _receive) = channel::<WindowStatus>();

    let (render_thread, to_renderer) = spawn_renderer();

    spawn_window("Title 1", send.clone(), to_renderer.clone());
    spawn_window("Title 2", send.clone(), to_renderer.clone());

    std::mem::drop(to_renderer);

    render_thread.join().unwrap();
}

pub fn spawn_renderer() -> (JoinHandle<()>, Sender<renderer::RenderMessage>) {
    let (to_renderer, from_windows) = channel::<renderer::RenderMessage>();
    let joiner = spawn(move || {
        let renderer = renderer::Renderer::new();

        while let Ok(message) = from_windows.recv() {
            match message {
                renderer::RenderMessage::Empty => {
                    println!("RM");
                }
                renderer::RenderMessage::Submit {
                    fence,
                    wait_semaphore,
                    signal_semaphore,
                    ack,
                    commands,
                } => {
                    renderer.submit(commands, wait_semaphore, signal_semaphore, fence);
                    ack.send(renderer::RenderResponse::CommandsSubmitted).unwrap();
                }
            }
        }
    });

    (joiner, to_renderer)
}

pub fn spawn_window(
    title: &str,
    _ack_send: Sender<WindowStatus>,
    to_renderer: Sender<renderer::RenderMessage>,
) -> JoinHandle<()> {
    // let context = WindowContext::new();
    // let ui_state = ui_state::new();
    let (to_window, from_renderer) = sync_channel::<renderer::RenderResponse>(1);
    let title = title.to_owned();
    spawn(|| {
        window::window(title, move |control, event| {
            match event {
                WindowEvent::Created { window: _, size: _ } => {
                    // context = WindowContext::init();

                    // request renderer information
                        // pipeline layout, shader modules

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
                    return EventLoopControl::Stop;
                }
                WindowEvent::CloseRequested { window } => {
                    control.destroy(window);
                }
                WindowEvent::Resized { window, size } => {
                    // let frame = get_swapchain_image()
                    // ui.resize(size);

                    // could this be integrated into WindowEvent::Update?
                }
                WindowEvent::Update {} => {
                    to_renderer.send(renderer::RenderMessage::Empty).unwrap();

                    /*
                    // let frame = get_swapchain_image()

                    // ui_state.update(...);
                    // let ui_builder = UiBuilder::new(frame.vertex_buffer, frame.index_buffer, &mut ui_state);
                    // ui_callback(&ui_builder);

                    let request = frame.to_submit_request(to_window);

                    to_renderer.send(request);
                    */

                    match from_renderer.recv() {
                        Ok(response) => {}
                        Err(_) => unreachable!("Renderer closed contact pipe!")
                    }
                }
                _ => {}
            }
            EventLoopControl::Continue
        });
    })
}
