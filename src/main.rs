mod handle;
mod ui;

fn main() {
    let mut windows = Vec::new();
    windows.push(ui::os_window::OsWindow::new("Title 1"));
    windows.push(ui::os_window::OsWindow::new("Title 2"));
    windows.push(ui::os_window::OsWindow::new("Title 3"));
    windows.push(ui::os_window::OsWindow::new("Title 4"));
    windows.push(ui::os_window::OsWindow::new("Title 5"));

    while !windows.is_empty() {
        windows.retain(|window| !window.borrow().was_close_requested);

        ui::os_window::poll_events();
    }
}

/*
Sketch A: Simple Text + Button

fn main() {
    let gui = Gui::new();

    let mut viewport;
    let mut pressed;

    gui.run(|ui, dt| {
        viewport = ui.begin("Sketch A", viewport);

        let layout = viewport.push_layout(ui::Layout::Centered);
        layout.place(ui::Text("Hello there!"));
        pressed = layout.place(ui::Button("Push here", pressed));
        ui.end();
    });
}

*/
