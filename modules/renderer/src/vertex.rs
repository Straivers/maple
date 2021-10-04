use ash::vk;

use crate::{color::Color, geometry::float2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: float2,
    pub color: Color,
}

impl Vertex {
    pub const BINDING_DESCRIPTION: vk::VertexInputBindingDescription = vk::VertexInputBindingDescription {
        binding: 0,
        stride: std::mem::size_of::<Vertex>() as u32,
        input_rate: vk::VertexInputRate::VERTEX,
    };

    pub const ATTRIBUTE_DESCRIPTION: [vk::VertexInputAttributeDescription; 2] = [
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: 0,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: vk::Format::R8G8B8A8_UNORM,
            offset: std::mem::size_of::<float2>() as u32,
        },
    ];
}

impl crate::geometry::Rect {
    /// Converts a `Rect2D` into a set of vertices and associated indices. The
    /// vertices are listed clockwise from the lower-left corner, and the
    /// indices in clockwise rotation, bottom-left to top-right.
    ///
    /// 3---2 2
    /// |  / /|
    /// | / / |
    /// |/ /  |
    /// 0 0---1
    ///
    /// Indices: 0 1 2 2 3 0
    pub fn to_vertices(&self, color: Color) -> ([Vertex; 4], [u16; 6]) {
        let vertices = [
            Vertex {
                position: self.position,
                color,
            },
            Vertex {
                position: self.position + float2(self.width(), 0.0),
                color,
            },
            Vertex {
                position: self.position + self.extent,
                color,
            },
            Vertex {
                position: self.position + float2(0.0, self.height()),
                color,
            },
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        (vertices, indices)
    }
}
