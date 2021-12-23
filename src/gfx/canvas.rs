use crate::shapes::{Extent, Rect};

use super::{Color, Vertex};

pub struct Canvas {
    size: Extent,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Canvas {
    pub fn new(size: Extent) -> Self {
        Self {
            size,
            vertices: vec![],
            indices: vec![],
        }
    }

    pub fn size(&self) -> Extent {
        self.size
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u16] {
        &self.indices
    }
}

pub trait Draw<T> {
    fn draw(&mut self, shape: &T);
}

pub trait DrawStyled<T> {
    fn draw_styled(&mut self, shape: &T, color: Color);
}

impl DrawStyled<Rect> for Canvas {
    fn draw_styled(&mut self, shape: &Rect, color: Color) {
        let offset = self.vertices.len() as u16;

        for point in &shape.points() {
            self.vertices.push(Vertex {
                position: (point.x.into(), point.y.into()),
                color,
            });
        }

        for index in &Rect::INDICES {
            self.indices.push(offset + index);
        }
    }
}
