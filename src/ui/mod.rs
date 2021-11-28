mod layout;

use crate::{
    gfx::{Color, Extent, Point, Rect, Vertex},
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

#[derive(Default)]
pub struct Context {
    cursor: Point<f32>,
    window_size: Extent<f32>,
    prev_cursor: Point<f32>,
    was_clicked: bool,
}

impl Context {
    pub fn advance_frame(&mut self) {
        self.prev_cursor = self.cursor;
        self.was_clicked = false;
    }

    pub fn update_click(&mut self) {
        self.was_clicked = true;
    }

    pub fn update_cursor(&mut self, x: f32, y: f32) {
        self.cursor = Point::new(x, y);
    }

    pub fn update_window_size(&mut self, width: f32, height: f32) {
        self.window_size = Extent::new(width, height);
    }
}

pub struct Builder<'a> {
    context: &'a mut Context,
    region: Region,
}

impl<'a> Builder<'a> {
    pub fn new(context: &'a mut Context) -> Self {
        Self {
            context,
            region: Region::new(Color::rgba(0, 0, 0, 0), 0.0, LayoutDirection::LeftToRight),
        }
    }

    pub fn cursor(&self) -> Point<f32> {
        self.context.cursor
    }

    pub fn was_clicked(&self) -> bool {
        self.context.was_clicked
    }

    pub fn region(&mut self, region: Region) {
        self.region.push(region);
    }

    pub fn build(
        self,
        vertex_buffer: &'a mut dyn CountingOutputIter<Vertex>,
        index_buffer: &'a mut dyn OutputIter<u16>,
    ) {
        let bounds = Rect {
            lower_left_corner: Point::default(),
            extent: self.context.window_size,
        };
        self.region
            .write_buffers(bounds, vertex_buffer, index_buffer);
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LayoutDirection {
    LeftToRight,
    TopToBottom,
}

#[derive(Clone)]
pub struct Region {
    color: Color,
    margin: f32,
    layout_direction: LayoutDirection,
    children: Vec<Region>,
}

impl Region {
    pub fn new(color: Color, margin: f32, layout_direction: LayoutDirection) -> Self {
        Self {
            color,
            margin,
            layout_direction,
            children: vec![],
        }
    }

    pub fn with_children(
        color: Color,
        margin: f32,
        layout_direction: LayoutDirection,
        children: &[Region],
    ) -> Self {
        Self {
            color,
            margin,
            layout_direction,
            children: children.to_vec(),
        }
    }

    pub fn push(&mut self, region: Region) {
        self.children.push(region);
    }

    pub fn write_buffers(
        &self,
        bounds: Rect<f32>,
        vertex_buffer: &mut dyn CountingOutputIter<Vertex>,
        index_buffer: &mut dyn OutputIter<u16>,
    ) {
        Self::write_rect(bounds, self.color, vertex_buffer, index_buffer);

        if !self.children.is_empty() {
            match self.layout_direction {
                LayoutDirection::LeftToRight => {
                    let total_margin_width = self.margin * (self.children.len() + 1) as f32;
                    let per_box_width =
                        (bounds.width() - total_margin_width) / self.children.len() as f32;

                    let mut increasing_x = bounds.x() + self.margin;

                    let y = bounds.y() + self.margin;
                    let height = bounds.height() - 2.0 * self.margin;

                    assert!(height > 0.0);

                    for child in &self.children {
                        let child_bounds = Rect::new(increasing_x, y, per_box_width, height);
                        child.write_buffers(child_bounds, vertex_buffer, index_buffer);
                        increasing_x += per_box_width + self.margin;
                    }
                }
                LayoutDirection::TopToBottom => {
                    let total_margin_height = self.margin * (self.children.len() + 1) as f32;
                    let per_box_height =
                        (bounds.height() - total_margin_height) / self.children.len() as f32;

                    // going bottom-up
                    let mut increasing_y = bounds.y() + self.margin;

                    let x = bounds.x() + self.margin;
                    let width = bounds.width() - 2.0 * self.margin;

                    for child in self.children.iter().rev() {
                        let child_bounds = Rect::new(x, increasing_y, width, per_box_height);
                        child.write_buffers(child_bounds, vertex_buffer, index_buffer);
                        increasing_y += per_box_height + self.margin;
                    }
                }
            }
        }
    }

    fn write_rect(
        rect: Rect<f32>,
        color: Color,
        vertex_buffer: &mut dyn CountingOutputIter<Vertex>,
        index_buffer: &mut dyn OutputIter<u16>,
    ) {
        let start = vertex_buffer.len() as u16;

        for point in &rect.points() {
            vertex_buffer.push(Vertex {
                position: *point,
                color,
            });
        }
        for index in &Rect::<f32>::INDICES {
            index_buffer.push(start + index);
        }
    }
}
