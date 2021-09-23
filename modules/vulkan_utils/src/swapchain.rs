use super::context::{load_vk_objects, Context};

use ash::vk;
const PREFERRED_SWAPCHAIN_LENGTH: u32 = 3;

#[derive(Debug, Default)]
pub struct SwapchainData {
    /// The format of the swapchain's images.
    pub format: vk::Format,

    pub color_space: vk::ColorSpaceKHR,

    /// The method by which the images are presented in the swapchain.
    pub present_mode: vk::PresentModeKHR,

    /// A handle to the swapchain, managed by the Vulkan drivers.
    pub handle: vk::SwapchainKHR,

    pub image_size: vk::Extent2D,

    /// The images used by the swapchain.
    pub images: Vec<vk::Image>,

    image_index: Option<u32>,
}

impl SwapchainData {
    /// Creates a new swapchain for the given window, as well as associated
    /// semaphores needed when acquiring and presenting swapchain images.
    ///
    /// # Errors
    /// Swapchain creation may fail for the following reasons:
    ///
    /// - `VK_ERROR_OUT_OF_HOST_MEMORY`
    /// - `VK_ERROR_OUT_OF_DEVICE_MEMORY`
    /// - `VK_ERROR_SURFACE_LOST`
    /// - `VK_ERROR_DEVICE_LOST`
    /// - `VK_ERROR_NATIVE_WINDOW_IN_USE_KHR`
    /// - `VK_ERROR_INITIALIZATION_FAILED`
    ///
    /// In addition to fallible Vulkan API calls, this function will also return
    /// `VK_ERROR_NATIVE_WINDOW_IN_USE_KHR` if the passed `WindowRef` is not
    /// valid.
    ///
    /// # Panics
    /// This function assumes that the initialized device in `context` was
    /// tested for surface support through platform-specific methods (e.g. the
    /// `vkGetPhysicalDeviceWin32PresentationSupportKHR` function), and will
    /// panic if the device does not support creating `VkSurface`s.
    pub fn new(context: &mut Context, surface: vk::SurfaceKHR, extent: vk::Extent2D) -> Self {
        // We test with platform-specific APIs during surface creation
        assert!(unsafe {
            context.surface_api.get_physical_device_surface_support(
                context.gpu.handle,
                context.gpu.present_queue_index,
                surface,
            )
        }
        .unwrap_or(false));

        context.create_or_resize_swapchain(surface, extent, None)
    }

    pub fn destroy(self, context: &mut Context) {
        unsafe {
            context.swapchain_api.destroy_swapchain(self.handle, None);
        }
    }

    pub fn get_image(&mut self, context: &Context, acquire_semaphore: vk::Semaphore) -> Option<u32> {
        context.get_swapchain_image(self, acquire_semaphore)
    }

    pub fn present(&mut self, context: &Context, wait_semaphores: &[vk::Semaphore]) -> bool {
        context.present_swapchain_image(self, wait_semaphores)
    }

    pub fn resize(&mut self, context: &Context, surface: vk::SurfaceKHR, extent: vk::Extent2D) {
        *self =
            context.create_or_resize_swapchain(surface, extent, Some((self.handle, std::mem::take(&mut self.images))));
    }
}

impl Context {
    #[cfg(target_os = "windows")]
    #[must_use]
    pub fn create_surface(&self, window_handle: sys::window_handle::WindowHandle) -> vk::SurfaceKHR {
        let ci = vk::Win32SurfaceCreateInfoKHR::builder()
            .hwnd(window_handle.hwnd)
            .hinstance(window_handle.hinstance);
        unsafe { self.os_surface_api.create_win32_surface(&ci, None) }.expect("Out of memory")
    }

    #[cfg(target_os = "windows")]
    pub fn destroy_surface(&self, surface: vk::SurfaceKHR) {
        unsafe {
            self.surface_api.destroy_surface(surface, None);
        }
    }

    #[must_use]
    pub fn create_or_resize_swapchain(
        &self,
        surface: vk::SurfaceKHR,
        size: vk::Extent2D,
        old: Option<(vk::SwapchainKHR, Vec<vk::Image>)>,
    ) -> SwapchainData {
        let capabilities = unsafe {
            self.surface_api
                .get_physical_device_surface_capabilities(self.gpu.handle, surface)
                .unwrap()
        };

        let format = {
            let formats = load_vk_objects::<_, _, 64>(|count, ptr| unsafe {
                self.surface_api
                    .fp()
                    .get_physical_device_surface_formats_khr(self.gpu.handle, surface, count, ptr)
            })
            .unwrap();

            *formats
                .iter()
                .find(|f| f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .unwrap_or(&formats[0])
        };

        let present_mode = *load_vk_objects::<_, _, 8>(|count, ptr| unsafe {
            self.surface_api
                .fp()
                .get_physical_device_surface_present_modes_khr(self.gpu.handle, surface, count, ptr)
        })
        .unwrap()
        .iter()
        .find(|p| **p == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(&vk::PresentModeKHR::FIFO);

        let image_size = {
            if capabilities.current_extent.width == u32::MAX {
                vk::Extent2D {
                    width: size
                        .width
                        .clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                    height: size.height.clamp(
                        capabilities.min_image_extent.height,
                        capabilities.max_image_extent.height,
                    ),
                }
            } else {
                capabilities.current_extent
            }
        };

        let min_images = PREFERRED_SWAPCHAIN_LENGTH.clamp(capabilities.min_image_count, capabilities.max_image_count);

        let mut create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(min_images)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(image_size)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let queue_family_indices = [self.gpu.graphics_queue_index, self.gpu.present_queue_index];
        if queue_family_indices[0] == queue_family_indices[1] {
            create_info.image_sharing_mode = vk::SharingMode::EXCLUSIVE;
        } else {
            create_info.image_sharing_mode = vk::SharingMode::CONCURRENT;
            create_info.queue_family_index_count = 2;
            create_info.p_queue_family_indices = queue_family_indices.as_ptr();
        }

        let (old_swapchain, old_images) = old.unwrap_or((vk::SwapchainKHR::null(), Vec::new()));
        create_info.old_swapchain = old_swapchain;

        let handle = unsafe { self.swapchain_api.create_swapchain(&create_info, None) }.unwrap();

        if create_info.old_swapchain != vk::SwapchainKHR::null() {
            unsafe {
                self.swapchain_api.destroy_swapchain(create_info.old_swapchain, None);
            }
        }

        SwapchainData {
            handle,
            format: format.format,
            image_size,
            color_space: format.color_space,
            present_mode,
            images: self.get_swapchain_images(handle, old_images),
            image_index: None,
        }
    }

    pub fn destroy_swapchain(&self, swapchain: SwapchainData) {
        unsafe {
            self.swapchain_api.destroy_swapchain(swapchain.handle, None);
        }
        std::mem::drop(swapchain);
    }

    fn get_swapchain_images(&self, swapchain: vk::SwapchainKHR, mut buffer: Vec<vk::Image>) -> Vec<vk::Image> {
        let mut count = 0;
        unsafe {
            self.swapchain_api
                .fp()
                .get_swapchain_images_khr(self.device.handle(), swapchain, &mut count, std::ptr::null_mut())
                .result()
                .expect("Out of memory");
        }

        buffer.clear();
        buffer.reserve(count as usize);

        unsafe {
            self.swapchain_api
                .fp()
                .get_swapchain_images_khr(self.device.handle(), swapchain, &mut count, buffer.as_mut_ptr())
                .result()
                .expect("Out of memory");

            buffer.set_len(count as usize);
        }

        buffer
    }

    fn get_swapchain_image(&self, swapchain: &mut SwapchainData, acquire_semaphore: vk::Semaphore) -> Option<u32> {
        match unsafe {
            self.swapchain_api
                .acquire_next_image(swapchain.handle, u64::MAX, acquire_semaphore, vk::Fence::null())
        } {
            Ok((index, _)) => {
                swapchain.image_index = Some(index);
                swapchain.image_index
            }
            Err(vkr) => match vkr {
                vk::Result::ERROR_OUT_OF_DATE_KHR => None,
                any => panic!("Unexpected error {:?}", any),
            },
        }
    }

    fn present_swapchain_image(&self, swapchain: &mut SwapchainData, wait_semaphores: &[vk::Semaphore]) -> bool {
        let swapchains = [swapchain.handle];
        let indices = [swapchain.image_index.expect("Did not acquire image before presenting")];
        swapchain.image_index = None;

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&indices);

        match unsafe { self.swapchain_api.queue_present(self.graphics_queue, &present_info) } {
            Ok(update) => update,
            Err(err) => {
                if err == vk::Result::ERROR_OUT_OF_DATE_KHR {
                    true
                } else {
                    panic!("Unexpected error: {:?}", err)
                }
            }
        }
    }
}
