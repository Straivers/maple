//! Maple Engine entry point

// shaders need to be built every time they change...
// applications don't always know which shaders they're going to need ahead of time
// shaders should be compiled ahead of time for release
// shaders should be recompiled on command during debug
// maple runner is a debug-only tool right now, can afford runtime compilation

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use clap::{App, Arg};

use renderer::{
    color::Color,
    geometry::{float2, Rect},
    vertex::Vertex,
    Renderer, WindowContext,
};

use sys::{
    dpi::PhysicalSize,
    window::{EventLoop, EventLoopControl},
    window_event::WindowEvent,
    window_handle::WindowHandle,
};

const MIN_FRAME_RATE: u32 = 60;

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

struct AppWindow {
    size: PhysicalSize,
    swapchain: WindowContext<Vertex>,
    last_draw: Instant,
}

struct AppState {
    windows: HashMap<WindowHandle, AppWindow>,
    renderer: Renderer,
}

impl AppState {
    fn new(extra_validation: bool) -> Self {
        let vulkan_lib =
            sys::library::Library::load("vulkan-1").expect("Could not initialize Vulkan, vulkan-1 not found");
        Self {
            windows: HashMap::new(),
            renderer: Renderer::new(vulkan_lib, extra_validation),
        }
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        for (_, app_window) in self.windows.drain() {
            self.renderer.destroy_swapchain(app_window.swapchain);
        }
    }
}

fn run(cli_options: &CliOptions) {
    let rectangle = Rect {
        position: float2(100.0, 100.0),
        extent: float2(400.0, 100.0),
    };

    let (vertices, indices) = rectangle.to_vertices(Color::rgb(128, 128, 128));

    let mut app_state = AppState::new(cli_options.with_vulkan_validation);
    let min_frame_time = Duration::from_secs(1) / MIN_FRAME_RATE;

    let mut event_loop = EventLoop::new(move |proxy, event| {
        match event {
            WindowEvent::Created { window, size } => {
                println!("Created");
                let swapchain = app_state.renderer.create_swapchain(window, size);
                app_state.windows.insert(
                    window,
                    AppWindow {
                        size,
                        swapchain,
                        last_draw: Instant::now(),
                    },
                );
            }
            WindowEvent::CloseRequested { window } => {
                println!("Destroyed");
                if let Some(app_window) = app_state.windows.remove(&window) {
                    app_state.renderer.destroy_swapchain(app_window.swapchain);
                }
                proxy.destroy_window(window);

                if proxy.num_windows() == 0 {
                    return EventLoopControl::Stop;
                }
            }
            WindowEvent::Resized { window, size } => {
                let app_window = app_state.windows.get_mut(&window).unwrap();
                app_window.size = size;
                app_state
                    .renderer
                    .render_to(&mut app_window.swapchain, size, &vertices, &indices);
                app_window.last_draw = Instant::now();

                // Keep other windows from locking up whle modalling resizing.
                for (handle, app_window) in &mut app_state.windows {
                    if *handle != window && Instant::now() - app_window.last_draw >= min_frame_time {
                        app_state
                            .renderer
                            .render_to(&mut app_window.swapchain, app_window.size, &vertices, &indices);
                        app_window.last_draw = Instant::now();
                    }
                }
            }
            WindowEvent::Redraw {} => {
                for app_window in app_state.windows.values_mut() {
                    app_state
                        .renderer
                        .render_to(&mut app_window.swapchain, app_window.size, &vertices, &indices);
                    app_window.last_draw = Instant::now();
                }

                app_state.renderer.end_frame();
            }
            WindowEvent::Update {} => {}
        }
        EventLoopControl::Continue
    });

    let _ = event_loop.create_window("Title 1");
    let _ = event_loop.create_window("Title 2");
    event_loop.run(20);
}
