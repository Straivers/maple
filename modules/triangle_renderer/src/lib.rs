use sys::library::Library;


type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InternalError(Box<dyn std::error::Error>)
}

#[doc(hidden)]
impl From<vulkan::Error> for Error {
    fn from(vkr: vulkan::Error) -> Self {
        Error::InternalError(Box::new(vkr))
    }
}

pub struct Swapchain{
    swapchain: vulkan::Swapchain,
    window: sys::window::WindowRef,
}

pub struct TriangleRenderer {
    vulkan: vulkan::Context,
}

impl TriangleRenderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Result<Self> {
        Ok(Self {
            vulkan: vulkan::Context::new(vulkan_library, debug_mode)?
        })
    }

    pub fn create_swapchain(&mut self, window: sys::window::WindowRef) -> Result<Swapchain> {
        Ok(Swapchain {
            swapchain: vulkan::Swapchain::new(&mut self.vulkan, &window)?,
            window
        })
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        swapchain.swapchain.destroy(&mut self.vulkan);
    }

    pub fn render_to(&mut self, swapchain: &mut Swapchain) {

    }
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {

    }
}
