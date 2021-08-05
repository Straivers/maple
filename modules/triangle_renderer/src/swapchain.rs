use std::rc::Rc;

use ash::vk;

use crate::constants::FRAMES_IN_FLIGHT;
use crate::effect::Effect;

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

/*
render pass depends on the format of the swapchain
    pipeline depends on render pass

look up render pass by format?

if swapchain format changes
    decrease refcount on render pass
    remove if 0

    create a new render pass with correct format
*/
