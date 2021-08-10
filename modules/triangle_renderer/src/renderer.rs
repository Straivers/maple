use ash::vk;
use sys::{dpi::PhysicalSize, library::Library};

use crate::constants::FRAMES_IN_FLIGHT;
use crate::effect::TriangleEffectBase;
use crate::error::{Error};
use crate::swapchain::Swapchain;

pub struct TriangleRenderer {
    vulkan: vulkan_utils::Context,
    effect_base: TriangleEffectBase,
}

impl TriangleRenderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Result<Self, Error> {
        let mut vulkan = vulkan_utils::Context::new(vulkan_library, debug_mode)?;
        let effect_base = TriangleEffectBase::new(&mut vulkan);

        Ok(Self { vulkan, effect_base })
    }

    pub fn create_swapchain(&mut self, window: sys::window::WindowRef) -> Swapchain {
        let surface = self.vulkan.create_surface(&window);
        let swapchain = {
            let extent = physical_size_to_extent(window.framebuffer_size().unwrap());
            vulkan_utils::SwapchainData::new(&mut self.vulkan, surface, extent)
        };
        let effect = self.effect_base.get_effect(&mut self.vulkan, swapchain.format);

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
                buffer.push(self.vulkan.create_image_view(&view_create_info));
            }
            buffer
        };

        let framebuffers = {
            let mut buffer = Vec::with_capacity(swapchain.images.len());
            for view in &image_views {
                let attachments = [*view];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(effect.render_pass)
                    .attachments(&attachments)
                    .width(swapchain.image_size.width)
                    .height(swapchain.image_size.height)
                    .layers(1);
                buffer.push(self.vulkan.create_framebuffer(&create_info));
            }
            buffer
        };

        Swapchain {
            current_frame: 0,
            surface,
            swapchain,
            window,
            presentation_effect: effect,
            image_views,
            framebuffers,
            sync_acquire: [
                self.vulkan.get_or_create_semaphore(),
                self.vulkan.get_or_create_semaphore(),
            ],
            sync_present: [
                self.vulkan.get_or_create_semaphore(),
                self.vulkan.get_or_create_semaphore(),
            ],
            sync_fence: [
                self.vulkan.get_or_create_fence(true),
                self.vulkan.get_or_create_fence(true),
            ],
            command_pools: [
                self.vulkan.create_graphics_command_pool(true),
                self.vulkan.create_graphics_command_pool(true),
            ],
        }
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        unsafe {
            self.vulkan
                .device
                .wait_for_fences(&swapchain.sync_fence, true, u64::MAX)
                .unwrap();
        }

        swapchain.swapchain.destroy(&mut self.vulkan);
        self.vulkan.destroy_surface(swapchain.surface);

        for view in swapchain.image_views {
            self.vulkan.destroy_image_view(view);
        }

        for framebuffer in swapchain.framebuffers {
            self.vulkan.destroy_framebuffer(framebuffer);
        }

        unsafe {
            for i in 0..FRAMES_IN_FLIGHT {
                self.vulkan
                    .device
                    .destroy_command_pool(swapchain.command_pools[i], None);
                self.vulkan.free_semaphore(swapchain.sync_acquire[i]);
                self.vulkan.free_semaphore(swapchain.sync_present[i]);
                self.vulkan.free_fence(swapchain.sync_fence[i]);
            }
        }
    }

    pub fn end_frame(&mut self) {
        self.effect_base.cleanup(&mut self.vulkan);
    }

    pub fn render_to(&mut self, swapchain: &mut Swapchain) {
        let frame = swapchain.frame_in_flight();

        if frame.extent == vk::Extent2D::default() {
            return;
        }

        if frame.was_resized {
            self.resize_swapchain(swapchain);
            return;
        }

        let _ = self.vulkan.wait_for_fences(&[frame.submit_fence], u64::MAX);

        let image_index = if let Some(index) = swapchain.swapchain.get_image(&self.vulkan, frame.acquire_semaphore) {
            index
        } else {
            self.resize_swapchain(swapchain);
            return;
        };

        let command_pool = swapchain.command_pools[swapchain.current_frame];
        self.vulkan.reset_command_pool(command_pool, false);

        let command_buffer = {
            let mut buffer = [vk::CommandBuffer::default()];
            self.vulkan.allocate_command_buffers(command_pool, &mut buffer);
            buffer[0]
        };

        let viewport_rect = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: frame.extent,
        };

        {
            let begin_info = vk::CommandBufferBeginInfo::default();
            unsafe { self.vulkan.device.begin_command_buffer(command_buffer, &begin_info) }.expect("Out of memory");
        }

        swapchain.presentation_effect.apply(
            &self.vulkan,
            swapchain.framebuffers[image_index as usize],
            viewport_rect,
            command_buffer,
        );

        unsafe {
            self.vulkan
                .device
                .end_command_buffer(command_buffer)
                .expect("Out of memory");
        }

        {
            let submit_info = vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: std::ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: &frame.acquire_semaphore,
                p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                signal_semaphore_count: 1,
                p_signal_semaphores: &frame.present_semaphore,
                command_buffer_count: 1,
                p_command_buffers: &command_buffer,
            };

            self.vulkan.reset_fences(&[frame.submit_fence]);
            self.vulkan.submit_to_graphics_queue(&[submit_info], frame.submit_fence);
        }

        if swapchain.swapchain.present(&self.vulkan, &[frame.present_semaphore]) {
            self.resize_swapchain(swapchain);
        }

        swapchain.current_frame = (swapchain.current_frame + 1) % FRAMES_IN_FLIGHT;
    }

    fn resize_swapchain(&mut self, swapchain: &mut Swapchain) {
        let framebuffer_extent = physical_size_to_extent(swapchain.window.framebuffer_size().unwrap());

        let _ = self.vulkan.wait_for_fences(&swapchain.sync_fence, u64::MAX);

        for pool in &swapchain.command_pools {
            self.vulkan.reset_command_pool(*pool, false);
        }

        swapchain
            .swapchain
            .resize(&self.vulkan, swapchain.surface, framebuffer_extent);

        swapchain.presentation_effect = self
            .effect_base
            .get_effect(&mut self.vulkan, swapchain.swapchain.format);

        assert!(swapchain.image_views.len() == swapchain.framebuffers.len());

        for (view, buffer) in swapchain.image_views.iter().zip(swapchain.framebuffers.iter()) {
            self.vulkan.destroy_image_view(*view);
            self.vulkan.destroy_framebuffer(*buffer);
        }

        swapchain.image_views.clear();
        swapchain.image_views.reserve(swapchain.swapchain.images.len());
        swapchain.framebuffers.clear();
        swapchain.framebuffers.reserve(swapchain.swapchain.images.len());

        for image in &swapchain.swapchain.images {
            let view_create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
                image: *image,
                format: swapchain.swapchain.format,
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

            let view = self.vulkan.create_image_view(&view_create_info);

            let attachments = [view];
            let create_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::FramebufferCreateFlags::empty(),
                render_pass: swapchain.presentation_effect.render_pass(),
                p_attachments: attachments.as_ptr(),
                attachment_count: 1,
                width: framebuffer_extent.width,
                height: framebuffer_extent.height,
                layers: 1,
            };

            swapchain.image_views.push(view);
            swapchain
                .framebuffers
                .push(self.vulkan.create_framebuffer(&create_info));
        }
    }
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {
        TriangleEffectBase::destroy(std::mem::take(&mut self.effect_base), &mut self.vulkan);
    }
}

fn physical_size_to_extent(size: PhysicalSize) -> vk::Extent2D {
    vk::Extent2D {
        width: u32::from(size.width),
        height: u32::from(size.height),
    }
}
