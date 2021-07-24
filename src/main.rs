mod ui;
use ui::os_window::*;

fn main() {
    let mut event_loop = EventLoop::new();

    let mut windows = Vec::new();
    windows.push(create_window(&event_loop, "Title 1"));
    windows.push(create_window(&event_loop, "Title 2"));
    windows.push(create_window(&event_loop, "Title 3"));
    windows.push(create_window(&event_loop, "Title 4"));
    windows.push(create_window(&event_loop, "Title 5"));

    while !windows.is_empty() {
        event_loop.poll();
        windows.retain(|window| !window.was_close_requested());
    }
}

fn create_window(event_loop: &EventLoop, title: &str) -> Box<Window> {
    Window::new(event_loop, title)
}
