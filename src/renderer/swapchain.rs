use super::context::VulkanContext;
use super::error::{RendererError, RendererResult};
use pal::vulkan::vk;
#[cfg(target_os = "windows")]
use pal::win32::{Foundation::HWND, System::LibraryLoader::GetModuleHandleW};
use std::mem::ManuallyDrop;

union SwapchainFrameResource<T: vk::Handle> {
    // These are Vulkan handles, so we don't need to drop them
    inline: ManuallyDrop<[T; 3]>,
    heap: ManuallyDrop<Vec<T>>,
}

pub struct Swapchain {
    /// The number of images used by the swapchain.
    num_images: u8,

    /// The index of the current frame into the frame resources.
    frame_index: u8,

    /// The format of the swapchain's images.
    format: vk::Format,

    /// The method by which the images are presented in the swapchain.
    present_mode: vk::PresentModeKHR,

    /// A handle to the surface that represents the swapchain's window.
    surface: vk::SurfaceKHR,

    /// A handle to the swapchain, managed by the Vulkan drivers.
    handle: vk::SwapchainKHR,

    /// The images used by the swapchain.
    images: SwapchainFrameResource<vk::Image>,

    /// Views of the swapchain images; one per image.
    views: SwapchainFrameResource<vk::ImageView>,

    /// Semaphores used to indicate when a swapchain image is ready to be
    /// rendered to.
    acquire_semaphores: SwapchainFrameResource<vk::Semaphore>,

    /// Semaphores used to indicate when a swapchain image is ready to be
    /// presented to the screen.
    release_semaphores: SwapchainFrameResource<vk::Semaphore>,
}

impl Swapchain {
    #[cfg(target_os = "windows")]
    pub fn new_win32(context: &VulkanContext, hwnd: HWND) -> RendererResult<Self> {
        let surface = {
            let hinstance = unsafe { GetModuleHandleW(None) };
            let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
                .hwnd(hwnd.0 as _)
                .hinstance(hinstance.0 as _);

            unsafe {
                context
                    .os_surface_api
                    .create_win32_surface(&create_info, None)
            }?
        };



        todo!()
    }

    pub(crate) fn destroy(self, context: &VulkanContext) {
        unsafe { context.surface_api.destroy_surface(self.surface, None) };
    }
}

struct SwapchainProperties {
    format: vk::Format,
    extent: vk::Extent2D,
    color_space: vk::ColorSpaceKHR,
    present_mode: vk::PresentModeKHR,
}

#[test]
fn test() {
    use std::mem::size_of;
    assert_eq!(
        size_of::<SwapchainFrameResource<vk::Image>>(),
        3 * size_of::<usize>()
    );
    assert_eq!(
        size_of::<SwapchainFrameResource<vk::ImageView>>(),
        3 * size_of::<usize>()
    );
}
