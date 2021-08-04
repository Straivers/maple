use super::context::{load_vk_objects, Context};
use super::error::{Error, Result};
use sys::window::WindowRef;

use ash::vk;

/// Triple buffering
const FRAMES_IN_FLIGHT: u32 = 2;
const MAX_SWAPCHAIN_IMAGES: usize = 32;

#[derive(Debug, Default)]
struct SwapchainImage {
    image: vk::Image,
    view: vk::ImageView,
}

#[derive(Debug)]
struct SwapchainSync {
    acquire_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
}

#[derive(Debug)]
pub struct Swapchain {
    /// The format of the swapchain's images.
    format: vk::Format,

    color_space: vk::ColorSpaceKHR,

    /// The method by which the images are presented in the swapchain.
    present_mode: vk::PresentModeKHR,

    /// A handle to the surface that represents the swapchain's window.
    surface: vk::SurfaceKHR,

    /// A handle to the swapchain, managed by the Vulkan drivers.
    handle: vk::SwapchainKHR,

    /// The images used by the swapchain.
    images: Vec<SwapchainImage>,

    sync_objects: [SwapchainSync; FRAMES_IN_FLIGHT as usize],
}

impl Swapchain {
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
    pub fn new(context: &mut Context, window: &WindowRef) -> Result<Self> {
        let surface = create_surface(context, window)?;

        // We test with platform-specific APIs during surface creation
        assert!(unsafe {
            context.surface_api.get_physical_device_surface_support(
                context.gpu.handle,
                context.gpu.present_queue_index,
                surface,
            )
        }?);

        let capabilities = unsafe {
            context
                .surface_api
                .get_physical_device_surface_capabilities(context.gpu.handle, surface)?
        };

        let format = {
            let formats = load_vk_objects::<_, _, 64>(|count, ptr| unsafe {
                context.surface_api.fp().get_physical_device_surface_formats_khr(
                    context.gpu.handle,
                    surface,
                    count,
                    ptr,
                )
            })?;

            *formats
                .iter()
                .find(|f| f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .unwrap_or(&formats[0])
        };

        let present_mode = *load_vk_objects::<_, _, 8>(|count, ptr| unsafe {
            context.surface_api.fp().get_physical_device_surface_present_modes_khr(
                context.gpu.handle,
                surface,
                count,
                ptr,
            )
        })?
        .iter()
        .find(|p| **p == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(&vk::PresentModeKHR::FIFO);

        let extent = {
            if capabilities.current_extent.width == u32::MAX {
                if let Some(size) = window.framebuffer_size() {
                    vk::Extent2D {
                        width: u32::from(size.width)
                            .clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                        height: u32::from(size.height).clamp(
                            capabilities.min_image_extent.height,
                            capabilities.max_image_extent.height,
                        ),
                    }
                } else {
                    return Err(Error::NativeWindowInUse);
                }
            } else {
                capabilities.current_extent
            }
        };

        let min_images = FRAMES_IN_FLIGHT.clamp(capabilities.min_image_count, capabilities.max_image_count);

        let swapchain = {
            let mut create_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(surface)
                .min_image_count(min_images)
                .image_format(format.format)
                .image_color_space(format.color_space)
                .image_extent(extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .pre_transform(capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true);

            let queue_family_indices = [context.gpu.graphics_queue_index, context.gpu.present_queue_index];

            create_info = if queue_family_indices[0] == queue_family_indices[1] {
                create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            } else {
                create_info
                    .image_sharing_mode(vk::SharingMode::CONCURRENT)
                    .queue_family_indices(&queue_family_indices)
            };

            unsafe { context.swapchain_api.create_swapchain(&create_info, None) }?
        };

        let mut images = Vec::new();
        get_swapchain_images(context, swapchain, format.format, &mut images)?;

        let sync_objects = [
            SwapchainSync {
                acquire_semaphore: context.get_or_create_semaphore()?,
                present_semaphore: context.get_or_create_semaphore()?,
            },
            SwapchainSync {
                acquire_semaphore: context.get_or_create_semaphore()?,
                present_semaphore: context.get_or_create_semaphore()?,
            },
        ];

        Ok(Swapchain {
            format: format.format,
            color_space: format.color_space,
            present_mode,
            surface,
            handle: swapchain,
            images,
            sync_objects,
        })
    }

    pub fn destroy(self, context: &mut Context) {
        unsafe {
            for image in self.images {
                context.device.destroy_image_view(image.view, None);
            }

            for sync in self.sync_objects {
                context.free_semaphore(sync.acquire_semaphore);
                context.free_semaphore(sync.present_semaphore);
            }

            context.swapchain_api.destroy_swapchain(self.handle, None);
            context.surface_api.destroy_surface(self.surface, None);
        }
    }
}

#[cfg(target_os = "windows")]
fn create_surface(context: &Context, window: &WindowRef) -> Result<vk::SurfaceKHR> {
    if let Some(handle) = window.handle() {
        let ci = vk::Win32SurfaceCreateInfoKHR::builder()
            .hwnd(handle.hwnd)
            .hinstance(handle.hinstance);
        Ok(unsafe { context.os_surface_api.create_win32_surface(&ci, None) }?)
    } else {
        Err(Error::NativeWindowInUse)
    }
}

fn get_swapchain_images(
    context: &Context,
    swapchain: vk::SwapchainKHR,
    format: vk::Format,
    buffer: &mut Vec<SwapchainImage>,
) -> Result<()> {
    let images = load_vk_objects::<_, _, MAX_SWAPCHAIN_IMAGES>(|count, ptr| unsafe {
        context
            .swapchain_api
            .fp()
            .get_swapchain_images_khr(context.device.handle(), swapchain, count, ptr)
    })?;

    for slot in buffer.iter_mut() {
        assert_ne!(slot.view, vk::ImageView::default());
        unsafe {
            context.device.destroy_image_view(slot.view, None);
        }
    }

    buffer.clear();
    buffer.reserve(images.len());

    let mut view_create_info = vk::ImageViewCreateInfo::builder()
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .components(vk::ComponentMapping::default())
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        })
        .build();

    for image in &images {
        let view = {
            view_create_info.image = *image;
            unsafe { context.device.create_image_view(&view_create_info, None) }?
        };

        buffer.push(SwapchainImage { image: *image, view });
    }

    Ok(())
}
