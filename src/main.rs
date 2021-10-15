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

use tokio;

mod window;

#[derive(Debug)]
struct CliOptions {
    with_vulkan_validation: bool,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    run(&options).await
}

async fn run(cli_options: &CliOptions) -> Result<(), Box<dyn std::error::Error>> {
    let (send, mut receive) = tokio::sync::mpsc::channel::<bool>(64);
    let closer = tokio::spawn(async move {
        let mut counter = 0;

        while let Some(v) = receive.recv().await {
            if v  {
                counter += 1;
            }
            else {
                counter -= 1;
            }

            if counter == 0 {
                receive.close();
                break;
            }
        }
    });

    spawn_window("Title 1", send.clone());
    // spawn_window("Title 2", send.clone());

    Ok(closer.await?)
}

pub fn spawn_window(title: &str, ack_send: tokio::sync::mpsc::Sender<bool>) -> tokio::task::JoinHandle<()> {
    // let context = None;
    // let ui = ui_builder::new();
    // let ui_state = ui_state::new();
    // let to_renderer = renderer.channel();
    let title = title.to_owned();
    tokio::task::spawn_blocking(|| {
        window::window(title, move |control, event| {
            match event {
                WindowEvent::Created { window, size } => {
                    // let (send, recv) = oneshot::channel();
                    // to_renderer.send(RendererMessage::NewWindow{ send });

                    // match recv.recv() {
                    //     RendererMessage::WindowContext { r_context } => {
                    //         context = r_context;
                    //     }
                    //     _ => panic!("Unexpected renderer message!")
                    // }
                    ack_send.blocking_send(true).unwrap();
                }
                WindowEvent::Destroyed { window } => {
                    // to_renderer.blocking_send(RendererMessage::WindowDestroyed{}).unwrap();
                    ack_send.blocking_send(false).unwrap();
                    return EventLoopControl::Stop
                }
                WindowEvent::Redraw {} => {
                    // let render_request = context.unwrap().make_request(ui.vertices, ui.indices);

                    // let (send, recv) = oneshot::channel();
                    // to_renderer.send(RendererMessage::RenderRequest{ render_request, send });

                    // match recv.recv() {
                    //     RendererMessage::RenderComplete => {}
                    //     _ => panic!("Unexpected renderer message!")
                    // }
                }
                _ => {}
            }
            EventLoopControl::Continue
        });
    })
}
