#![allow(dead_code)]

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

pub struct AppData {}

/// The [`AppDataTracker`] wraps [`AppData`] in order to keep track of which panel
/// accesses which data items. This is used in conjunction with input state to
/// determine if a panel needs to be updated.
pub struct AppDataTracker {}

pub struct MapleUi {}

impl MapleUi {
    pub fn panel(&mut self, panel_control: &dyn Fn(&mut Panel)) {}
}

pub struct Panel {}

impl Panel {
    pub fn data(&self) -> &AppDataTracker {
        todo!()
    }

    pub fn data_mut(&mut self) -> &mut AppDataTracker {
        todo!()
    }

    pub fn text(&mut self, text: &str) {}

    pub fn button(&mut self, label: &str) -> bool {
        false
    }
}
