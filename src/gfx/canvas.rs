use crate::shapes::{Extent, Rect};

use super::{Color, Vertex};

#[derive(Default)]
pub struct CanvasStorage {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

pub struct Canvas<'a> {
    size: Extent,
    storage: &'a mut CanvasStorage,
}

impl<'a> Canvas<'a> {
    pub fn new(size: Extent, storage: &'a mut CanvasStorage) -> Self {
        storage.vertices.clear();
        storage.indices.clear();

        Self { size, storage }
    }

    pub fn clear(&mut self) {
        self.storage.vertices.clear();
        self.storage.indices.clear();
    }

    pub fn size(&self) -> Extent {
        self.size
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.storage.vertices
    }

    pub fn indices(&self) -> &[u16] {
        &self.storage.indices
    }
}

pub trait Draw<T> {
    fn draw(&mut self, shape: &T);
}

pub trait DrawStyled<T> {
    fn draw_styled(&mut self, shape: &T, color: Color);
}

impl<'a> DrawStyled<Rect> for Canvas<'a> {
    fn draw_styled(&mut self, shape: &Rect, color: Color) {
        let offset = self.storage.vertices.len() as u16;

        for point in &shape.points() {
            self.storage.vertices.push(Vertex {
                position: (point.x.into(), point.y.into()),
                color,
            });
        }

        for index in &Rect::INDICES {
            self.storage.indices.push(offset + index);
        }
    }
}
