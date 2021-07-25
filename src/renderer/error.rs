use pal::vulkan::vk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererError {
    LibraryNotFound(&'static str),
    VulkanError(vk::Result),
    NoSuitableGPU,
}

impl From<vk::Result> for RendererError {
    fn from(vkr: vk::Result) -> Self {
        RendererError::VulkanError(vkr)
    }
}