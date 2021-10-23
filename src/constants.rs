pub const TRIANGLE_VERTEX_SHADER_SPIRV: &[u8] = include_bytes!("../shaders/simple_vertex_vert.spv");
pub const TRIANGLE_FRAGMENT_SHADER_SPIRV: &[u8] = include_bytes!("../shaders/simple_vertex_frag.spv");
pub const FRAMES_IN_FLIGHT: usize = 2;
pub const DEFAULT_VERTEX_BUFFER_SIZE: usize = 8192;
pub const MAX_SWAPCHAIN_DEPTH: usize = 8;
