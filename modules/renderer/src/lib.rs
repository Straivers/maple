mod constants;
mod effect;

pub mod color;
pub mod geometry;
pub mod vertex;

mod window_context;
pub use window_context::WindowContext;

mod triangle_renderer;
pub use triangle_renderer::TriangleRenderer;
