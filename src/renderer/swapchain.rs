use super::context::{load_vk_objects, VulkanContext};
use super::error::RendererResult;
use crate::window::Window;

use pal::vulkan::{
    vk::{self, Handle},
    DeviceV1_0,
};
#[cfg(target_os = "windows")]
use pal::win32::System::LibraryLoader::GetModuleHandleW;
use std::convert::TryInto;
use std::mem::ManuallyDrop;

/// Triple buffering
const NUM_FRAMEBUFFERS: usize = 3;

union SwapchainFrameResource<T: vk::Handle> {
    // These are Vulkan handles, so we don't need to drop them
    inline: ManuallyDrop<[T; NUM_FRAMEBUFFERS]>,
    heap: ManuallyDrop<Vec<T>>,
}

impl<T: vk::Handle> SwapchainFrameResource<T> {
    fn get_slice(&self, count: usize) -> &[T] {
        if count <= NUM_FRAMEBUFFERS {
            unsafe { &self.inline[0..count] }
        } else {
            unsafe { self.heap.as_slice() }
        }
    }
}

pub struct Swapchain {
    /// The number of images used by the swapchain.
    num_images: u8,

    /// The index of the current frame into the frame resources.
    frame_index: u8,

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
    images: SwapchainFrameResource<vk::Image>,

    /// Views of the swapchain images; one per image.
    views: SwapchainFrameResource<vk::ImageView>,
    // Semaphores used to indicate when a swapchain image is ready to be
    // rendered to.
    // acquire_semaphores: SwapchainFrameResource<vk::Semaphore>,

    // Semaphores used to indicate when a swapchain image is ready to be
    // presented to the screen.
    // release_semaphores: SwapchainFrameResource<vk::Semaphore>,
}

impl Swapchain {
    pub fn new(context: &VulkanContext, window: &Window) -> RendererResult<Self> {
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
                context
                    .surface_api
                    .fp()
                    .get_physical_device_surface_formats_khr(
                        context.gpu.handle,
                        surface,
                        count,
                        ptr,
                    )
            })?;

            *formats
                .iter()
                .find(|f| {
                    f.format == vk::Format::B8G8R8A8_SRGB
                        && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                })
                .unwrap_or(&formats[0])
        };

        let present_mode = *load_vk_objects::<_, _, 8>(|count, ptr| unsafe {
            context
                .surface_api
                .fp()
                .get_physical_device_surface_present_modes_khr(
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
                let size = window.framebuffer_size();
                vk::Extent2D {
                    width: size.width.clamp(
                        capabilities.min_image_extent.width,
                        capabilities.max_image_extent.width,
                    ),
                    height: size.height.clamp(
                        capabilities.min_image_extent.height,
                        capabilities.max_image_extent.height,
                    ),
                }
            } else {
                capabilities.current_extent
            }
        };

        let min_images = (NUM_FRAMEBUFFERS as u32)
            .clamp(capabilities.min_image_count, capabilities.max_image_count);

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

            let queue_family_indices = [
                context.gpu.graphics_queue_index,
                context.gpu.present_queue_index,
            ];

            create_info = if queue_family_indices[0] == queue_family_indices[1] {
                create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            } else {
                create_info
                    .image_sharing_mode(vk::SharingMode::CONCURRENT)
                    .queue_family_indices(&queue_family_indices)
            };

            unsafe { context.swapchain_api.create_swapchain(&create_info, None) }?
        };

        let mut num_images = 0;
        unsafe {
            context.swapchain_api.fp().get_swapchain_images_khr(
                context.device.handle(),
                swapchain,
                &mut num_images,
                std::ptr::null_mut(),
            )
        }
        .result()?;

        let images = get_swapchain_frame_resource(num_images, |slice| unsafe {
            let mut count = slice.len().try_into().unwrap();
            context
                .swapchain_api
                .fp()
                .get_swapchain_images_khr(
                    context.device.handle(),
                    swapchain,
                    &mut count,
                    slice.as_mut_ptr(),
                )
                .result()?;
            Ok(())
        })?;

        println!("{:?}", images.get_slice(num_images as usize));

        let views = get_swapchain_frame_resource(num_images, |slice| unsafe {
            let mut create_info = vk::ImageViewCreateInfo::builder()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format.format)
                .components(vk::ComponentMapping::default())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build();

            for (i, img) in images.get_slice(num_images as usize).iter().enumerate() {
                create_info.image = *img;
                slice[i] = context.device.create_image_view(&create_info, None)?;
            }
            Ok(())
        })?;

        Ok(Swapchain {
            num_images: num_images.try_into().expect("Too many swapchain images"),
            frame_index: 0,
            format: format.format,
            color_space: format.color_space,
            present_mode,
            surface,
            handle: swapchain,
            images,
            views,
        })
    }

    pub(crate) fn destroy(self, context: &VulkanContext) {
        unsafe {
            for view in self.views.get_slice(self.num_images as usize) {
                context.device.destroy_image_view(*view, None);
            }

            context.swapchain_api.destroy_swapchain(self.handle, None);
            context.surface_api.destroy_surface(self.surface, None);
        }
    }
}

#[cfg(target_os = "windows")]
fn create_surface(context: &VulkanContext, window: &Window) -> RendererResult<vk::SurfaceKHR> {
    let hinstance = unsafe { GetModuleHandleW(None) };
    let ci = vk::Win32SurfaceCreateInfoKHR::builder()
        .hwnd(window.get_hwnd().0 as _)
        .hinstance(hinstance.0 as _);
    Ok(unsafe { context.os_surface_api.create_win32_surface(&ci, None) }?)
}

struct SwapchainProperties {
    format: vk::Format,
    extent: vk::Extent2D,
    color_space: vk::ColorSpaceKHR,
    present_mode: vk::PresentModeKHR,
}

fn get_swapchain_frame_resource<
    T: Handle + Default + Copy,
    F: FnMut(&mut [T]) -> RendererResult<()>,
>(
    required_size: u32,
    mut func: F,
) -> RendererResult<SwapchainFrameResource<T>> {
    let size = required_size as usize;
    if size <= NUM_FRAMEBUFFERS {
        let mut buffer = [T::default(); NUM_FRAMEBUFFERS];
        func(&mut buffer[0..size])?;
        Ok(SwapchainFrameResource::<T> {
            inline: ManuallyDrop::new(buffer),
        })
    } else {
        let mut buffer = Vec::with_capacity(size);
        func(&mut buffer[0..size])?;
        unsafe { buffer.set_len(size) };
        Ok(SwapchainFrameResource::<T> {
            heap: ManuallyDrop::new(buffer),
        })
    }
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
