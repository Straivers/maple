mod layout;
mod tree;
mod widgets;

pub use layout::{compute_layout, LayoutTree, Region};
pub use tree::Index;
pub use widgets::{Panel, Widget, WidgetTree, WidgetTreeBuilder};

/*
Desired API (2021-11-06):

let mut data = AppData::new(...);

// Pass data here, because we want to track which widgets access which data
// members.
ui.build(&mut data, |ui: UiBuilder| {
    ui.panel(|panel| {
        if panel.button("Hi!") {
            if let Some(count) = panel.get_mut<u32>("ui/button_press_count") {
                count += 1;
            }
            else {
                panel.insert<u32>("ui/button_press_count", 1);
            }
        }
    });
    ui.panel(|panel| {

    });
});
*/
