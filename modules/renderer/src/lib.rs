mod constants;
mod effect;

pub mod color;
pub mod geometry;
pub mod vertex;

mod swapchain;
pub use swapchain::Swapchain;

mod triangle_renderer;
pub use triangle_renderer::TriangleRenderer;
