mod renderer;
mod window;

use clap::{App, Arg};
use window::{EventLoop, Window};

#[derive(Debug)]
struct CliOptions {
    enable_vulkan_validation: bool,
}

fn main() {
    let matches = App::new("maple")
        .version("0.1.0")
        .version_short("v")
        .arg(
            Arg::with_name("enable_vulkan_validation")
                .long_help("Toggles vulkan validation layers. You must have a recent installation of the Vulkan SDK. This is true by default in debug builds.")
                .long("enable-vulkan-validation")
                .takes_value(true)
                .possible_values(&["true", "false"]),
        )
        .get_matches();

    let options = CliOptions {
        enable_vulkan_validation: {
            if let Some(enable) = matches.value_of("enable_vulkan_validation") {
                enable.parse().unwrap()
            } else {
                cfg!(debug_assertions)
            }
        },
    };

    run(&options);
}

fn run(cli_options: &CliOptions) {
    let _vk_context =
        renderer::context::VulkanContext::new(cli_options.enable_vulkan_validation).unwrap();

    let mut event_loop = EventLoop::new();
    let mut windows = Vec::new();
    windows.push(create_window(&event_loop, "Title 1"));

    while !windows.is_empty() {
        event_loop.poll();

        windows.retain(|window| !window.was_close_requested());
    }
}

fn create_window(event_loop: &EventLoop, title: &str) -> Window {
    Window::new(event_loop, title)
}
