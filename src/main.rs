//! Maple Engine entry point

// shaders need to be built every time they change...
// applications don't always know which shaders they're going to need ahead of time
// shaders should be compiled ahead of time for release
// shaders should be recompiled on command during debug
// maple runner is a debug-only tool right now, can afford runtime compilation

use clap::{App, Arg};

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

struct AppWindow {
    window: windowing::Window,
    swapchain: Box<renderer::swapchain::Swapchain>,
}

struct AppState {
    event_loop: windowing::EventLoop,
    windows: Vec<AppWindow>,
    vulkan: renderer::context::VulkanContext,
}

impl AppState {
    fn new(extra_validation: bool) -> Self {
        Self {
            event_loop: windowing::EventLoop::new(),
            windows: Vec::new(),
            vulkan: renderer::context::VulkanContext::new(extra_validation).unwrap(),
        }
    }

    fn create_window(&mut self, title: &str) {
        let window = windowing::Window::new(&self.event_loop, title);
        let swapchain = Box::new(renderer::swapchain::Swapchain::new(&mut self.vulkan, &window).unwrap());
        self.windows.push(AppWindow { window, swapchain: swapchain });
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        for window in self.windows.drain(0..) {
            window.swapchain.destroy(&mut self.vulkan);
        }
    }
}

fn run(cli_options: &CliOptions) {
    let mut app_state = AppState::new(cli_options.enable_vulkan_validation);
    app_state.create_window("Title 1");
    app_state.create_window("Title 2");

    while !app_state.windows.is_empty() {
        app_state.event_loop.poll();

        /*
        for window in windows {
            if window.was_resized() {
                app_state.triangle_renderer.resize_swapchain(swapchain);
            }
        }
        */

        let mut i = 0;
        while i < app_state.windows.len() {
            if app_state.windows[i].window.was_close_requested() {
                let window = app_state.windows.swap_remove(i);
                window.swapchain.destroy(&mut app_state.vulkan);
            }
            else {
                i += 1;
            }
        }
    }
}
