//! Maple Engine entry point

// shaders need to be built every time they change...
// applications don't always know which shaders they're going to need ahead of time
    // shaders should be compiled ahead of time for release
    // shaders should be recompiled on command during debug
    // maple runner is a debug-only tool right now, can afford runtime compilation

use clap::{App, Arg};
use windowing::{EventLoop, Window};

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
    let mut vk_context = renderer::context::VulkanContext::new(cli_options.enable_vulkan_validation).unwrap();

    let mut event_loop = EventLoop::new();
    let mut windows = vec![create_window(&event_loop, "Title 1")];

    let swapchain = renderer::swapchain::Swapchain::new(&mut vk_context, &windows[0]).unwrap();
    // let pipeline = create_triangle_pipeline(swapchain.format)
    // let framebuffers = pipeline.create_framebuffers(swapchain.image_views);

    while !windows.is_empty() {
        event_loop.poll();

        /*
        for window in windows {
            if window.was_resized() {
                // pipelines are refcounted...?
            }
        }
        */

        windows.retain(|window| !window.was_close_requested());
    }

    // pipeline.free_framebuffers(framebuffers);
    // pipeline.destroy(&mut vk_context);
    swapchain.destroy(&mut vk_context);
}

fn create_window(event_loop: &EventLoop, title: &str) -> Window {
    Window::new(event_loop, title)
}

// Notes on renderer:
// let swapchain = renderer.create_swapchain(window)

// needs reference to renderer...?
// swapchain.resize()

// single renderer, multiple windows
    // renderer.submit_commands(swapchain, commands);
    // renderer.resize_swapchain(swapchain);
    // all swapchains have the same tickrate, framerate may vary
        // if rendering is not complete for the frame, skip