use crate::ui::os_window::OsWindow;

mod handle;
mod ui;

fn main() {
    let mut w = ui::os_window::OsWindow::new("title");
    println!("{:?}", w.as_mut() as *mut OsWindow);

    while !w.was_close_requested {
        ui::os_window::poll_events();
    }

    println!("done");

    std::mem::drop(w);
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
