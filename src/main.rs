//! Maple Engine entry point

// shaders need to be built every time they change...
// applications don't always know which shaders they're going to need ahead of time
// shaders should be compiled ahead of time for release
// shaders should be recompiled on command during debug
// maple runner is a debug-only tool right now, can afford runtime compilation

use clap::{App, Arg};
use triangle_renderer::{TriangleRenderer, Swapchain};

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
    window: sys::window::Window,
    swapchain: Box<Swapchain>,
}

struct AppState {
    event_loop: sys::window::EventLoop,
    windows: Vec<AppWindow>,
    renderer: TriangleRenderer
}

impl AppState {
    fn new(extra_validation: bool) -> Self {
        let vulkan_lib =
            sys::library::Library::load("vulkan-1").expect("Could not initialize Vulkan, vulkan-1 not found");
        Self {
            event_loop: sys::window::EventLoop::new(),
            windows: Vec::new(),
            renderer: TriangleRenderer::new(vulkan_lib, extra_validation).unwrap(),
        }
    }

    fn create_window(&mut self, title: &str) {
        let window = sys::window::Window::new(&self.event_loop, title);
        let swapchain = Box::new(self.renderer.create_swapchain(window.get()).unwrap());
        self.windows.push(AppWindow { window, swapchain });
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        for window in self.windows.drain(0..) {
            self.renderer.destroy_swapchain(*window.swapchain);
        }
    }
}

fn run(cli_options: &CliOptions) {
    let mut app_state = AppState::new(cli_options.enable_vulkan_validation);
    app_state.create_window("Title 1");
    app_state.create_window("Title 2");

    while !app_state.windows.is_empty() {
        app_state.event_loop.poll();

        let mut i = 0;
        while i < app_state.windows.len() {
            if app_state.windows[i].window.was_close_requested() {
                let window = app_state.windows.swap_remove(i);
                app_state.renderer.destroy_swapchain(*window.swapchain);
            } else {
                i += 1;
            }
        }

        for window in &mut app_state.windows {
            app_state.renderer.render_to(&mut window.swapchain);
        }
    }
}
