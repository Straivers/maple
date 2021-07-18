mod handle;
mod ui;

fn main() {
    let mut w1 = true;
    let mut w2 = true;

    let mut ui = ui::Context::new();
    ui.run(move |ui| {
        if w1 && ui.begin("main") {
            ui.end();
        } else {
            w1 = false;
        }

        if w2 && ui.begin("main2") {
            ui.end();
        } else {
            w2 = false;
        }
    });
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
