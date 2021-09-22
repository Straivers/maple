use std::rc::Rc;

use ash::vk;

use crate::constants::FRAMES_IN_FLIGHT;
use crate::effect::{Effect, EffectBase};
use sys::{dpi::PhysicalSize, window_handle::WindowHandle};

pub struct FrameInFlight {
    pub was_resized: bool,
    pub extent: vk::Extent2D,
    pub submit_fence: vk::Fence,
    pub command_pool: vk::CommandPool,
    pub acquire_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore,
}

pub struct Swapchain {
    pub current_frame: usize,
    pub surface: vk::SurfaceKHR,
    pub swapchain: vulkan_utils::SwapchainData,
    pub presentation_effect: Rc<dyn Effect>,
    pub image_views: Vec<vk::ImageView>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub sync_acquire: [vk::Semaphore; FRAMES_IN_FLIGHT],
    pub sync_present: [vk::Semaphore; FRAMES_IN_FLIGHT],
    pub sync_fence: [vk::Fence; FRAMES_IN_FLIGHT],
    pub command_pools: [vk::CommandPool; FRAMES_IN_FLIGHT],
}

impl Swapchain {
    pub fn new(
        context: &mut vulkan_utils::Context,
        window_handle: WindowHandle,
        framebuffer_size: PhysicalSize,
        presentation_effect: &mut dyn EffectBase,
    ) -> Self {
        let surface = context.create_surface(window_handle);
        let swapchain = {
            let extent = physical_size_to_extent(framebuffer_size);
            vulkan_utils::SwapchainData::new(context, surface, extent)
        };
        let effect = presentation_effect.get_effect(context, swapchain.format);

        let image_views = {
            let mut buffer = Vec::with_capacity(swapchain.images.len());
            for image in &swapchain.images {
                let view_create_info = vk::ImageViewCreateInfo::builder()
                    .image(*image)
                    .format(swapchain.format)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                buffer.push(context.create_image_view(&view_create_info));
            }
            buffer
        };

        let framebuffers = {
            let mut buffer = Vec::with_capacity(swapchain.images.len());
            for view in &image_views {
                let attachments = [*view];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(effect.render_pass())
                    .attachments(&attachments)
                    .width(swapchain.image_size.width)
                    .height(swapchain.image_size.height)
                    .layers(1);
                buffer.push(context.create_framebuffer(&create_info));
            }
            buffer
        };

        Swapchain {
            current_frame: 0,
            surface,
            swapchain,
            presentation_effect: effect,
            image_views,
            framebuffers,
            sync_acquire: [context.get_or_create_semaphore(), context.get_or_create_semaphore()],
            sync_present: [context.get_or_create_semaphore(), context.get_or_create_semaphore()],
            sync_fence: [context.get_or_create_fence(true), context.get_or_create_fence(true)],
            command_pools: [
                context.create_graphics_command_pool(true),
                context.create_graphics_command_pool(true),
            ],
        }
    }

    pub fn destroy(self, context: &mut vulkan_utils::Context) {
        let _ = context.wait_for_fences(&self.sync_fence, u64::MAX);

        self.swapchain.destroy(context);
        context.destroy_surface(self.surface);

        for view in self.image_views {
            context.destroy_image_view(view);
        }

        for framebuffer in self.framebuffers {
            context.destroy_framebuffer(framebuffer);
        }

        for i in 0..FRAMES_IN_FLIGHT {
            context.destroy_command_pool(self.command_pools[i]);
            context.free_semaphore(self.sync_acquire[i]);
            context.free_semaphore(self.sync_present[i]);
            context.free_fence(self.sync_fence[i]);
        }
    }

    pub fn resize(
        &mut self,
        fb_size: PhysicalSize,
        context: &mut vulkan_utils::Context,
        presentation_effect: &mut dyn EffectBase,
    ) {
        let framebuffer_extent = physical_size_to_extent(fb_size);

        let _ = context.wait_for_fences(&self.sync_fence, u64::MAX);

        for pool in &self.command_pools {
            context.reset_command_pool(*pool, false);
        }

        self.swapchain.resize(context, self.surface, framebuffer_extent);

        self.presentation_effect = presentation_effect.get_effect(context, self.swapchain.format);

        assert!(self.image_views.len() == self.framebuffers.len());

        for (view, buffer) in self.image_views.iter().zip(self.framebuffers.iter()) {
            context.destroy_image_view(*view);
            context.destroy_framebuffer(*buffer);
        }

        self.image_views.clear();
        self.image_views.reserve(self.swapchain.images.len());
        self.framebuffers.clear();
        self.framebuffers.reserve(self.swapchain.images.len());

        for image in &self.swapchain.images {
            let view_create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
                image: *image,
                format: self.swapchain.format,
                view_type: vk::ImageViewType::TYPE_2D,
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                },
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };

            let view = context.create_image_view(&view_create_info);

            let attachments = [view];
            let create_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::FramebufferCreateFlags::empty(),
                render_pass: self.presentation_effect.render_pass(),
                p_attachments: attachments.as_ptr(),
                attachment_count: 1,
                width: framebuffer_extent.width,
                height: framebuffer_extent.height,
                layers: 1,
            };

            self.image_views.push(view);
            self.framebuffers.push(context.create_framebuffer(&create_info));
        }
    }

    pub fn frame_in_flight(&self, target_size: PhysicalSize) -> FrameInFlight {
        let extent = physical_size_to_extent(target_size);
        FrameInFlight {
            was_resized: self.swapchain.image_size != extent,
            extent,
            submit_fence: self.sync_fence[self.current_frame],
            command_pool: self.command_pools[self.current_frame],
            acquire_semaphore: self.sync_acquire[self.current_frame],
            present_semaphore: self.sync_present[self.current_frame],
        }
    }
}

fn physical_size_to_extent(size: sys::dpi::PhysicalSize) -> vk::Extent2D {
    vk::Extent2D {
        width: u32::from(size.width),
        height: u32::from(size.height),
    }
}
