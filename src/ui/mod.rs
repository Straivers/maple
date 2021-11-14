#![allow(dead_code)]

use crate::{
    gfx::{Color, Rect, Vertex},
    traits::{CountingOutputIter, OutputIter},
};

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

/*
v1: Panel, click to change color
*/

pub struct Context {}

impl Context {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct Builder<'a> {
    context: &'a Context,
    window_rect: Rect,
    panels: Vec<Panel>,
    vertex_buffer: &'a mut dyn CountingOutputIter<Vertex>,
    index_buffer: &'a mut dyn OutputIter<u16>,
}

impl<'a> Builder<'a> {
    pub fn new(
        context: &'a Context,
        window_rect: Rect,
        vertex_buffer: &'a mut dyn CountingOutputIter<Vertex>,
        index_buffer: &'a mut dyn OutputIter<u16>,
    ) -> Self {
        Self {
            context,
            window_rect,
            vertex_buffer,
            index_buffer,
            panels: vec![],
        }
    }

    pub fn panel(&mut self, panel: Panel) {
        self.panels.push(panel);
    }

    pub fn build(self) {
        for panel in &self.panels {
            panel.write_buffers(self.vertex_buffer, self.index_buffer);
        }
    }
}

pub struct Panel {
    pub rect: Rect,
    pub color: Color,
}

impl Panel {
    pub fn write_buffers(
        &self,
        vertices: &mut dyn CountingOutputIter<Vertex>,
        indices: &mut dyn OutputIter<u16>,
    ) {
        let start = vertices.len() as u16;

        for point in &self.rect.points() {
            vertices.push(Vertex {
                position: *point,
                color: self.color,
            });
            
        }

        for index in &Rect::INDICES {
            indices.push(start + index);
        }
    }
}
