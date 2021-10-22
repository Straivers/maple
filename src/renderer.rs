//! This module contains types and functions for the render thread only, and
//! controls access to the graphics queue.
//!
//! Communication between the render thread and window threads occurs through
//! the types defined in the render_message module.

use ash::vk::{self, PresentInfoKHR};

use utils::array_vec::ArrayVec;

use crate::render_base::{Request, Response, VULKAN};

pub struct Renderer {
    signalled_fence_cache: ArrayVec<vk::Fence, 1024>,
    semaphore_cache: ArrayVec<vk::Semaphore, 1024>,
}

impl Renderer {
    pub fn new() -> Self {
        lazy_static::initialize(&VULKAN);

        Self {
            signalled_fence_cache: ArrayVec::new(),
            semaphore_cache: ArrayVec::new(),
        }
    }

    pub fn execute(&mut self, request: &Request) -> Response {
        match request {
            Request::ContextInit => Response::ContextInit {
                fences: [VULKAN.create_fence(true), VULKAN.create_fence(true)],
                wait_semaphores: [VULKAN.create_semaphore(), VULKAN.create_semaphore()],
                signal_semaphores: [VULKAN.create_semaphore(), VULKAN.create_semaphore()],
            },
            &Request::SubmitCommands {
                fence,
                wait_semaphore,
                signal_semaphore,
                commands,
                swapchain,
                image_id,
            } => {
                self.submit(commands, wait_semaphore, signal_semaphore, fence);

                let ci = PresentInfoKHR::builder()
                    .wait_semaphores(&[signal_semaphore])
                    .swapchains(&[swapchain])
                    .image_indices(&[image_id])
                    .build();
                unsafe { VULKAN.swapchain_api.queue_present(VULKAN.graphics_queue, &ci) };
                Response::CommandsSubmitted { image_id }
            }
        }
    }

    fn submit(&mut self, commands: vk::CommandBuffer, wait: vk::Semaphore, signal: vk::Semaphore, fence: vk::Fence) {
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &wait,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            signal_semaphore_count: 1,
            p_signal_semaphores: &signal,
            command_buffer_count: 1,
            p_command_buffers: &commands,
        };

        VULKAN.submit_to_graphics_queue(&[submit_info], fence);
    }
}
