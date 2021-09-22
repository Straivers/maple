//! Maple Engine entry point

// shaders need to be built every time they change...
// applications don't always know which shaders they're going to need ahead of time
// shaders should be compiled ahead of time for release
// shaders should be recompiled on command during debug
// maple runner is a debug-only tool right now, can afford runtime compilation

use clap::{App, Arg};
use renderer::{Swapchain, TriangleRenderer};
use sys::window::{EventLoop, Window};

#[derive(Debug)]
struct CliOptions {
    with_vulkan_validation: bool,
}

fn main() {
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

    run(&options);
}

type AppWindow = Window<Option<Swapchain>>;

struct AppState {
    event_loop: EventLoop<Option<Swapchain>>,
    windows: Vec<AppWindow>,
    renderer: TriangleRenderer,
}

impl AppState {
    fn new(extra_validation: bool) -> Self {
        let vulkan_lib =
            sys::library::Library::load("vulkan-1").expect("Could not initialize Vulkan, vulkan-1 not found");
        Self {
            event_loop: EventLoop::new(),
            windows: Vec::new(),
            renderer: TriangleRenderer::new(vulkan_lib, extra_validation),
        }
    }

    fn create_window(&mut self, title: &str) {
        let window = AppWindow::new(&self.event_loop, title, None);
        let swapchain = Some(
            self.renderer
                .create_swapchain(window.handle(), window.framebuffer_size()),
        );
        *window.data_mut() = swapchain;
        self.windows.push(window);
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        for window in self.windows.drain(0..) {
            if let Some(swapchain) = window.data_mut().take() {
                self.renderer.destroy_swapchain(swapchain);
            }
        }
    }
}

fn run(cli_options: &CliOptions) {
    let mut app_state = AppState::new(cli_options.with_vulkan_validation);
    app_state.create_window("Title 1");
    app_state.create_window("Title 2");

    while !app_state.windows.is_empty() {
        app_state.event_loop.poll();

        let mut i = 0;
        while i < app_state.windows.len() {
            if app_state.windows[i].was_close_requested() {
                let window = app_state.windows.swap_remove(i);

                if let Some(swapchain) = window.data_mut().take() {
                    app_state.renderer.destroy_swapchain(swapchain);
                };
            } else {
                i += 1;
            }
        }

        for window in &mut app_state.windows {
            let fb_size = window.framebuffer_size();
            if let Some(swapchain) = window.data_mut().as_mut() {
                app_state.renderer.render_to(swapchain, fb_size);
            }
        }

        app_state.renderer.end_frame();
    }
}
