use std::rc::Rc;

use ash::vk;

use crate::constants::FRAMES_IN_FLIGHT;
use crate::effect::Effect;
use crate::error::{Error, Result};

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
    pub swapchain: vulkan_utils::Swapchain,
    pub window: sys::window::WindowRef,
    pub presentation_effect: Rc<dyn Effect>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub sync_acquire: [vk::Semaphore; FRAMES_IN_FLIGHT],
    pub sync_present: [vk::Semaphore; FRAMES_IN_FLIGHT],
    pub sync_fence: [vk::Fence; FRAMES_IN_FLIGHT],
    pub command_pools: [vk::CommandPool; FRAMES_IN_FLIGHT],
}

impl Swapchain {
    pub fn frame_in_flight(&self) -> Result<FrameInFlight> {
        let framebuffer_size = if let Some(size) = self.window.framebuffer_size() {
            size
        } else {
            return Err(Error::WindowNotValid);
        };

        let extent = vk::Extent2D {
            width: framebuffer_size.width.into(),
            height: framebuffer_size.height.into(),
        };

        Ok(FrameInFlight {
            was_resized: self.swapchain.image_size != extent,
            extent: extent,
            submit_fence: self.sync_fence[self.current_frame],
            command_pool: self.command_pools[self.current_frame],
            acquire_semaphore: self.sync_acquire[self.current_frame],
            present_semaphore: self.sync_present[self.current_frame],
        })
    }
}

/*
render pass depends on the format of the swapchain
    pipeline depends on render pass

look up render pass by format?

if swapchain format changes
    decrease refcount on render pass
    remove if 0

    create a new render pass with correct format
*/