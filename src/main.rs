//! Maple Engine entry point

use crate::{dpi::PhysicalSize, window_event::WindowEvent};
use render_base::Response;
use render_context::RenderContext;

use clap::App;
use window::EventLoopControl;

use std::{
    sync::mpsc::{channel, sync_channel, Sender, SyncSender},
    thread::{spawn, JoinHandle},
};

use crate::render_base::to_extent;

mod array_vec;
mod color;
mod constants;
mod dpi;
mod geometry;
mod library;
mod recorder;
mod render_base;
mod render_context;
mod renderer;
mod swapchain;
mod vulkan;
mod window;
mod window_event;
mod window_handle;

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
    let (send, _receive) = channel::<WindowStatus>();

    let (render_thread, to_renderer) = spawn_renderer();

    spawn_window("Title 1", send.clone(), to_renderer.clone());
    spawn_window("Title 2", send, to_renderer.clone());

    std::mem::drop(to_renderer);

    render_thread.join().unwrap();
}

pub fn spawn_renderer() -> (JoinHandle<()>, Sender<(render_base::Request, SyncSender<Response>)>) {
    let (to_renderer, from_windows) = channel::<(render_base::Request, SyncSender<Response>)>();

    let joiner = spawn(move || {
        let mut renderer = renderer::Renderer::new();
        while let Ok((message, response)) = from_windows.recv() {
            response.send(renderer.execute(&message)).unwrap();
        }
    });

    (joiner, to_renderer)
}

pub fn spawn_window(
    title: &str,
    _ack_send: Sender<WindowStatus>,
    to_renderer: Sender<(render_base::Request, SyncSender<Response>)>,
) -> JoinHandle<()> {
    // We need at least 1 slot to buffer messages from the renderer so that the
    // renderer won't block waiting for the window thread to wake.
    let (to_window, from_renderer) = sync_channel::<render_base::Response>(1);
    let title = title.to_owned();
    spawn(move || {
        let mut context = RenderContext::new();
        let mut window_size = PhysicalSize { width: 0, height: 0 };

        window::window(title, |control, event| {
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

                    if let Some(request) = context.draw(to_extent(window_size), &vertices, &indices) {
                        to_renderer.send((request, to_window.clone())).unwrap();
                        let _ = from_renderer.recv();
                    }
                }
                WindowEvent::Update {} => {
                    let vertices = [];
                    let indices = [];

                    if let Some(request) = context.draw(to_extent(window_size), &vertices, &indices) {
                        to_renderer.send((request, to_window.clone())).unwrap();
                        let _ = from_renderer.recv();
                    }
                }
                _ => {}
            }
            EventLoopControl::Continue
        });
    })
}
