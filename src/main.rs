mod ui;

fn main() {
    let mut ui = ui::Context::new();
    ui.run(move |ui| {
        ui.begin("main");
        ui.end();

        ui.begin("main2");
        ui.begin("main3");
        ui.begin("main4");
        ui.begin("main5");
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
