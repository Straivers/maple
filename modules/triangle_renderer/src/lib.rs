use ash::vk;
use sys::library::Library;

mod constants;
mod effect;
mod error;
mod swapchain;

use constants::*;
pub use error::{Error, Result};
pub use swapchain::Swapchain;

pub struct TriangleRenderer {
    vulkan: vulkan_utils::Context,
    effect_base: effect::TriangleEffectBase,
}

impl TriangleRenderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Result<Self> {
        let mut vulkan = vulkan_utils::Context::new(vulkan_library, debug_mode)?;
        let effect_base = effect::TriangleEffectBase::new(&mut vulkan)?;

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

                buffers.push(unsafe { self.vulkan.device.create_framebuffer(&create_info, None) }?);
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
                self.vulkan.get_or_create_semaphore()?,
                self.vulkan.get_or_create_semaphore()?,
            ],
            sync_present: [
                self.vulkan.get_or_create_semaphore()?,
                self.vulkan.get_or_create_semaphore()?,
            ],
            sync_fence: [
                self.vulkan.get_or_create_fence(true)?,
                self.vulkan.get_or_create_fence(true)?,
            ],
            command_pools: [
                self.vulkan.create_graphics_command_pool(true)?,
                self.vulkan.create_graphics_command_pool(true)?,
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

        unsafe {
            self.vulkan
                .device
                .wait_for_fences(&[frame.submit_fence], true, u64::MAX)?;
        }

        let image_index = {
            let (index, update) = match swapchain.swapchain.get_image(&self.vulkan, frame.acquire_semaphore) {
                Ok((index, _)) => (index, false),
                Err(err) => {
                    if err == vulkan_utils::Error::SwapchainOutOfDate {
                        (0, true)
                    } else {
                        return Err(Error::from(err));
                    }
                }
            };

            if update {
                self.resize_swapchain(swapchain)?;
                return Ok(());
            }

            index
        };

        let command_pool = swapchain.command_pools[swapchain.current_frame];

        unsafe {
            self.vulkan
                .device
                .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())?;
        }

        let command_buffer = {
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();

            let mut buffer = vk::CommandBuffer::default();
            unsafe {
                self.vulkan.device.fp_v1_0().allocate_command_buffers(
                    self.vulkan.device.handle(),
                    &alloc_info,
                    &mut buffer,
                )
            }
            .result()?;
            buffer
        };

        let viewport_rect = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: frame.extent,
        };

        {
            let begin_info = vk::CommandBufferBeginInfo::default();
            unsafe { self.vulkan.device.begin_command_buffer(command_buffer, &begin_info) }?
        }

        swapchain.presentation_effect.apply(
            &self.vulkan,
            swapchain.framebuffers[image_index as usize],
            viewport_rect,
            command_buffer,
        );

        unsafe {
            self.vulkan.device.end_command_buffer(command_buffer)?;
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

            unsafe {
                self.vulkan.device.reset_fences(&[frame.submit_fence])?;
                self.vulkan
                    .device
                    .queue_submit(self.vulkan.graphics_queue, &[submit_info], frame.submit_fence)?;
            }
        }

        let present_status = {
            let wait = [frame.present_semaphore];
            let swapchains = [swapchain.swapchain.handle];
            let image_indices = [image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&wait)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            unsafe {
                self.vulkan
                    .swapchain_api
                    .queue_present(self.vulkan.graphics_queue, &present_info)
            }
        };
        
        if match present_status {
            Ok(update) => update,
            Err(err) => {
                if err == vk::Result::ERROR_OUT_OF_DATE_KHR {
                    true
                } else {
                    return Err(Error::from(err));
                }
            }
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

        unsafe {
            // We explicitly don't reset fences here, because we need them to be
            // signaled when we try to render again.
            self.vulkan.device.wait_for_fences(&swapchain.sync_fence, true, u64::MAX)?;
        }

        for pool in &swapchain.command_pools {
            unsafe {
                self.vulkan
                    .device
                    .reset_command_pool(*pool, vk::CommandPoolResetFlags::empty())?;
            }
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
                .push(unsafe { self.vulkan.device.create_framebuffer(&create_info, None) }?);
        }

        Ok(())
    }
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {
        effect::TriangleEffectBase::destroy(std::mem::take(&mut self.effect_base), &mut self.vulkan);
    }
}
