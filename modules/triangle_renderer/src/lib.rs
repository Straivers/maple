use sys::library::Library;

type Result<T> = std::result::Result<T, Error>;

const TRIANGLE_VERTEX_SHADER: &[u8] = include_bytes!("../shaders/tri.vert.spv");
const TRIANGLE_FRAGMENT_SHADER: &[u8] = include_bytes!("../shaders/tri.frag.spv");

#[derive(Debug)]
pub enum Error {
    InternalError(Box<dyn std::error::Error>),
}

#[doc(hidden)]
impl From<vulkan_utils::Error> for Error {
    fn from(vkr: vulkan_utils::Error) -> Self {
        Error::InternalError(Box::new(vkr))
    }
}

pub struct Swapchain {
    swapchain: vulkan_utils::Swapchain,
    window: sys::window::WindowRef,
}

pub struct TriangleRenderer {
    vulkan: vulkan_utils::Context,
}

impl TriangleRenderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Result<Self> {
        let mut vulkan = vulkan_utils::Context::new(vulkan_library, debug_mode)?;

        let shaders = [
            vulkan.create_shader(TRIANGLE_VERTEX_SHADER)?,
            vulkan.create_shader(TRIANGLE_FRAGMENT_SHADER)?,
        ];

        // create pipline
        // create render_pass

        vulkan.destroy_shader(shaders[0]);
        vulkan.destroy_shader(shaders[1]);

        Ok(Self { vulkan })
    }

    pub fn create_swapchain(&mut self, window: sys::window::WindowRef) -> Result<Swapchain> {
        Ok(Swapchain {
            swapchain: vulkan_utils::Swapchain::new(&mut self.vulkan, &window)?,
            window,
        })
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        swapchain.swapchain.destroy(&mut self.vulkan);
    }

    pub fn render_to(&mut self, swapchain: &mut Swapchain) {}
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {}
}
