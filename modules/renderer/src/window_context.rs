use std::rc::Rc;

use ash::vk;

use crate::constants::FRAMES_IN_FLIGHT;
use crate::effect::{Effect, EffectBase};
use sys::{dpi::PhysicalSize, window_handle::WindowHandle};

#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub image_view: vk::ImageView,
    pub frame_buffer: vk::Framebuffer,
    pub command_buffer: vk::CommandBuffer,
}

#[derive(Debug, Clone, Copy)]
pub struct FrameSync {
    pub fence: vk::Fence,
    pub acquire_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore,
}

impl Frame {
    fn new(
        context: &mut vulkan_utils::Context,
        image: vk::Image,
        image_size: vk::Extent2D,
        image_format: vk::Format,
        command_pool: vk::CommandPool,
        effect: &Rc<dyn Effect>,
    ) -> Frame {
        let image_view = {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(image)
                .format(image_format)
                .view_type(vk::ImageViewType::TYPE_2D)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            context.create_image_view(&create_info)
        };

        let frame_buffer = {
            let attachment = [image_view];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(effect.render_pass())
                .attachments(&attachment)
                .width(image_size.width)
                .height(image_size.height)
                .layers(1);

            context.create_frame_buffer(&create_info)
        };

        let command_buffer = {
            let mut buffers = [vk::CommandBuffer::null()];
            context.allocate_command_buffers(command_pool, &mut buffers);
            buffers[0]
        };

        Frame {
            image_view,
            frame_buffer,
            command_buffer,
        }
    }

    fn destroy(self, context: &mut vulkan_utils::Context, command_pool: vk::CommandPool) {
        context.destroy_image_view(self.image_view);
        context.destroy_frame_buffer(self.frame_buffer);
        context.free_command_buffers(command_pool, &[self.command_buffer]);
    }
}

pub struct WindowContext {
    pub current_frame: usize,
    pub surface: vk::SurfaceKHR,
    pub swapchain: vulkan_utils::SwapchainData,
    pub presentation_effect: Rc<dyn Effect>,
    pub frames: Vec<Frame>,
    pub command_pool: vk::CommandPool,
    pub sync_objects: [FrameSync; FRAMES_IN_FLIGHT],
}

impl WindowContext {
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

        let command_pool = context.create_graphics_command_pool(true, true);

        let frames = {
            let mut buffer = Vec::with_capacity(swapchain.images.len());
            for image in &swapchain.images {
                buffer.push(Frame::new(
                    context,
                    *image,
                    swapchain.image_size,
                    swapchain.format,
                    command_pool,
                    &effect,
                ))
            }

            buffer
        };

        WindowContext {
            current_frame: 0,
            surface,
            swapchain,
            presentation_effect: effect,
            frames,
            command_pool,
            sync_objects: [
                FrameSync {
                    fence: context.get_or_create_fence(true),
                    acquire_semaphore: context.get_or_create_semaphore(),
                    present_semaphore: context.get_or_create_semaphore(),
                },
                FrameSync {
                    fence: context.get_or_create_fence(true),
                    acquire_semaphore: context.get_or_create_semaphore(),
                    present_semaphore: context.get_or_create_semaphore(),
                },
            ],
        }
    }

    pub fn destroy(mut self, context: &mut vulkan_utils::Context) {
        let fences = [self.sync_objects[0].fence, self.sync_objects[1].fence];
        let _ = context.wait_for_fences(&fences, u64::MAX);

        for frame in self.frames.drain(0..) {
            frame.destroy(context, self.command_pool);
        }

        self.swapchain.destroy(context);
        context.destroy_surface(self.surface);

        for sync in &self.sync_objects {
            context.free_fence(sync.fence);
            context.free_semaphore(sync.acquire_semaphore);
            context.free_semaphore(sync.present_semaphore);
        }

        context.destroy_command_pool(self.command_pool);
    }

    pub fn resize(
        &mut self,
        context: &mut vulkan_utils::Context,
        fb_size: PhysicalSize,
        presentation_effect: &mut dyn EffectBase,
    ) {
        let fences = [self.sync_objects[0].fence, self.sync_objects[1].fence];
        let _ = context.wait_for_fences(&fences, u64::MAX);

        let framebuffer_extent = physical_size_to_extent(fb_size);

        self.swapchain.resize(context, self.surface, framebuffer_extent);

        self.presentation_effect = presentation_effect.get_effect(context, self.swapchain.format);

        for frame in self.frames.drain(0..) {
            frame.destroy(context, self.command_pool);
        }
        assert!(self.frames.is_empty());
        self.frames.reserve(self.swapchain.images.len());

        for image in &self.swapchain.images {
            self.frames.push(Frame::new(
                context,
                *image,
                self.swapchain.image_size,
                self.swapchain.format,
                self.command_pool,
                &self.presentation_effect,
            ));
        }
    }

    /// Returns None if resizing failed
    pub fn frame_in_flight(
        &mut self,
        context: &mut vulkan_utils::Context,
        target_size: PhysicalSize,
        presentation_effect: &mut dyn EffectBase,
    ) -> Option<(Frame, FrameSync)> {
        let extent = physical_size_to_extent(target_size);

        let _ = context.wait_for_fences(&[self.sync_objects[self.current_frame].fence], u64::MAX);

        let sync_objects = self.sync_objects[self.current_frame];

        if extent != self.swapchain.image_size {
            self.resize(context, target_size, presentation_effect);
        }

        let image_index = if let Some(index) = self.swapchain.get_image(context, sync_objects.acquire_semaphore) {
            index
        } else {
            self.resize(context, target_size, presentation_effect);
            self.swapchain.get_image(context, sync_objects.acquire_semaphore)?
        };

        if self.swapchain.image_size == extent {
            Some((self.frames[image_index as usize], sync_objects))
        } else {
            None
        }
    }
}

pub fn physical_size_to_extent(size: sys::dpi::PhysicalSize) -> vk::Extent2D {
    vk::Extent2D {
        width: u32::from(size.width),
        height: u32::from(size.height),
    }
}
