mod layout;
mod tree;

use crate::{
    gfx::{Color, Vertex},
    px::Px,
    shapes::{Extent, Point, Rect},
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
    cursor: Point,
    window_size: Extent,
    prev_cursor: Point,
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

    pub fn update_cursor(&mut self, x: Px, y: Px) {
        self.cursor = Point::new(x, y);
    }

    pub fn update_window_size(&mut self, width: Px, height: Px) {
        self.window_size = Extent::new(width, height);
    }
}

pub struct Builder {
    region: Region,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            region: Region::new(Color::rgba(0, 0, 0, 0), Px(0), LayoutDirection::LeftToRight),
        }
    }

    pub fn region(&mut self, region: Region) {
        self.region.push(region);
    }

    pub fn build(
        self,
        window_size: Extent,
        vertex_buffer: &mut dyn CountingOutputIter<Vertex>,
        index_buffer: &mut dyn OutputIter<u16>,
    ) {
        let bounds = Rect {
            point: Point::default(),
            extent: window_size,
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
    margin: Px,
    layout_direction: LayoutDirection,
    children: Vec<Region>,
}

impl Region {
    pub fn new(color: Color, margin: Px, layout_direction: LayoutDirection) -> Self {
        Self {
            color,
            margin,
            layout_direction,
            children: vec![],
        }
    }

    pub fn with_children(
        color: Color,
        margin: Px,
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
        bounds: Rect,
        vertex_buffer: &mut dyn CountingOutputIter<Vertex>,
        index_buffer: &mut dyn OutputIter<u16>,
    ) {
        Self::write_rect(bounds, self.color, vertex_buffer, index_buffer);

        if !self.children.is_empty() {
            match self.layout_direction {
                LayoutDirection::LeftToRight => {
                    let total_margin_width = self.margin * (self.children.len() + 1) as i16;
                    let per_box_width =
                        (bounds.width() - total_margin_width) / self.children.len() as i16;

                    let mut increasing_x = bounds.x() + self.margin;

                    let y = bounds.y() + self.margin;
                    let height = bounds.height() - 2 * self.margin;

                    assert!(height > 0);

                    for child in &self.children {
                        let child_bounds = Rect::new(increasing_x, y, per_box_width, height);
                        child.write_buffers(child_bounds, vertex_buffer, index_buffer);
                        increasing_x += per_box_width + self.margin;
                    }
                }
                LayoutDirection::TopToBottom => {
                    let total_margin_height = self.margin * (self.children.len() + 1) as i16;
                    let per_box_height =
                        (bounds.height() - total_margin_height) / self.children.len() as i16;

                    // going bottom-up
                    let mut increasing_y = bounds.y() + self.margin;

                    let x = bounds.x() + self.margin;
                    let width = bounds.width() - 2 * self.margin;

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
        rect: Rect,
        color: Color,
        vertex_buffer: &mut dyn CountingOutputIter<Vertex>,
        index_buffer: &mut dyn OutputIter<u16>,
    ) {
        let start = vertex_buffer.len() as u16;

        for point in &rect.points() {
            vertex_buffer.push(Vertex {
                position: (point.x.into(), point.y.into()),
                color,
            });
        }
        for index in &Rect::INDICES {
            index_buffer.push(start + index);
        }
    }
}
