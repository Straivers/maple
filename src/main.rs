mod renderer;
mod window;

use window::{EventLoop, Window};

fn main() {
    let mut vk_context = renderer::context::VulkanContext::new().unwrap();

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
