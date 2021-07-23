mod ui;

mod os_window2;

fn main() {
    // let mut windows = Vec::new();
    // windows.push(create_window("Title 1"));
    // windows.push(create_window("Title 2"));
    // windows.push(create_window("Title 3"));
    // windows.push(create_window("Title 4"));
    // windows.push(create_window("Title 5"));

    // while !windows.is_empty() {
    //     for window in &mut windows {
    //         window.poll();
    //     }

    //     windows.retain(|window| !window.was_close_requested);
    // }

    let mut wm = os_window2::WindowManager::new();
    let w1 = wm.create_window("Title 1");

    while wm.has_windows() {
        if let Some(window) = wm.get(w1) {
            if window.was_close_requested {
                wm.destroy_window(w1);
            }
        }

        wm.poll();
    }
}

// fn create_window(title: &str) -> Box<ui::os_window::OsWindow> {
    // ui::os_window::OsWindow::new(title)
// }

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
