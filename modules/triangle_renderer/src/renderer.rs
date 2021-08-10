use ash::vk;
use sys::library::Library;

use crate::constants::FRAMES_IN_FLIGHT;
use crate::effect::TriangleEffectBase;
use crate::error::Error;
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
        Swapchain::new(&mut self.vulkan, window, &mut self.effect_base)
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        swapchain.destroy(&mut self.vulkan)
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
            swapchain.resize(&mut self.vulkan, &mut self.effect_base);
            return;
        }

        let _ = self.vulkan.wait_for_fences(&[frame.submit_fence], u64::MAX);

        let image_index = if let Some(index) = swapchain.swapchain.get_image(&self.vulkan, frame.acquire_semaphore) {
            index
        } else {
            swapchain.resize(&mut self.vulkan, &mut self.effect_base);
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
            swapchain.resize(&mut self.vulkan, &mut self.effect_base);
        }

        swapchain.current_frame = (swapchain.current_frame + 1) % FRAMES_IN_FLIGHT;
    }
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {
        TriangleEffectBase::destroy(std::mem::take(&mut self.effect_base), &mut self.vulkan);
    }
}
