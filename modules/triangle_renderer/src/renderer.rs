use ash::vk;
use sys::library::Library;

use crate::constants::FRAMES_IN_FLIGHT;
use crate::effect::TriangleEffectBase;
use crate::error::{Error, Result};
use crate::swapchain::Swapchain;

pub struct TriangleRenderer {
    vulkan: vulkan_utils::Context,
    effect_base: TriangleEffectBase,
}

impl TriangleRenderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Result<Self> {
        let mut vulkan = vulkan_utils::Context::new(vulkan_library, debug_mode)?;
        let effect_base = TriangleEffectBase::new(&mut vulkan)?;

        Ok(Self { vulkan, effect_base })
    }

    pub fn create_swapchain(&mut self, window: sys::window::WindowRef) -> Result<Swapchain> {
        let swapchain = vulkan_utils::Swapchain::new(&mut self.vulkan, &window, (FRAMES_IN_FLIGHT + 1) as u32)?;
        let effect = self.effect_base.get_effect(&mut self.vulkan, swapchain.format)?;

        let framebuffers = {
            let mut buffers = Vec::with_capacity(swapchain.images.len());
            let image_size = window.framebuffer_size().unwrap();

            for image in &swapchain.images {
                let attachments = [image.view];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(effect.render_pass)
                    .attachments(&attachments)
                    .width(image_size.width.into())
                    .height(image_size.height.into())
                    .layers(1);

                buffers.push(self.vulkan.create_framebuffer(&create_info));
            }
            buffers
        };

        Ok(Swapchain {
            current_frame: 0,
            swapchain,
            window,
            presentation_effect: effect,
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
        })
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        unsafe {
            self.vulkan
                .device
                .wait_for_fences(&swapchain.sync_fence, true, u64::MAX)
                .unwrap();
        }

        swapchain.swapchain.destroy(&mut self.vulkan);
        unsafe {
            for framebuffer in swapchain.framebuffers {
                self.vulkan.device.destroy_framebuffer(framebuffer, None);
            }

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

    pub fn render_to(&mut self, swapchain: &mut Swapchain) -> Result<()> {
        let frame = swapchain.frame_in_flight()?;

        if frame.extent == vk::Extent2D::default() {
            return Ok(())
        }

        self.vulkan.wait_for_fences(&[frame.submit_fence], u64::MAX);

        let image_index = if let Ok(Some(index)) = swapchain.swapchain.get_image(&self.vulkan, frame.acquire_semaphore)
        {
            index
        } else {
            self.resize_swapchain(swapchain)?;
            return Ok(());
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
            self.vulkan.device.end_command_buffer(command_buffer).expect("Out of memory");
        }

        {
            let wait = [frame.acquire_semaphore];
            let signal = [frame.present_semaphore];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let commands = [command_buffer];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&wait)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&commands)
                .signal_semaphores(&signal)
                .build();

            self.vulkan.reset_fences(&[frame.submit_fence]);
            self.vulkan.submit_to_graphics_queue(&[submit_info], frame.submit_fence);
        }

        if {
            let wait = [frame.present_semaphore];
            swapchain.swapchain.present(&self.vulkan, &wait)
        } {
            self.resize_swapchain(swapchain)?;
        }

        swapchain.current_frame = (swapchain.current_frame + 1) % FRAMES_IN_FLIGHT;
        Ok(())
    }

    fn resize_swapchain(&mut self, swapchain: &mut Swapchain) -> Result<()> {
        let framebuffer_size = if let Some(size) = swapchain.window.framebuffer_size() {
            size
        } else {
            return Err(Error::WindowNotValid);
        };

        let framebuffer_extent = vk::Extent2D {
            width: framebuffer_size.width.into(),
            height: framebuffer_size.height.into(),
        };

        self.vulkan.wait_for_fences(&swapchain.sync_fence, u64::MAX);

        for pool in &swapchain.command_pools {
            self.vulkan.reset_command_pool(*pool, false);
        }

        let old_format = swapchain.swapchain.format;
        swapchain.swapchain.resize(&self.vulkan, framebuffer_extent)?;

        if old_format != swapchain.swapchain.format {
            swapchain.presentation_effect = self
                .effect_base
                .get_effect(&mut self.vulkan, swapchain.swapchain.format)?;
        }

        for framebuffer in swapchain.framebuffers.drain(..) {
            unsafe {
                self.vulkan.device.destroy_framebuffer(framebuffer, None);
            }
        }

        for image in &swapchain.swapchain.images {
            let attachments = [image.view];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(swapchain.presentation_effect.render_pass())
                .attachments(&attachments)
                .width(framebuffer_extent.width)
                .height(framebuffer_extent.height)
                .layers(1);

            swapchain
                .framebuffers
                .push(self.vulkan.create_framebuffer(&create_info));
        }

        Ok(())
    }
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {
        TriangleEffectBase::destroy(std::mem::take(&mut self.effect_base), &mut self.vulkan);
    }
}
