
// struct or trait?
// trait would allow substitution... do we need that?
    // struct at first, transition to trait only if absolutely necessary

use std::{sync::mpsc::Sender, time::Duration};


use ash::vk;

#[derive(Debug)]
pub enum RenderResponse {
    FramePresented
}

#[derive(Debug)]
pub enum RenderMessage {
    Empty,
    SubmitAndPresent {
        ack: Sender<RenderResponse>,

        fence: vk::Fence,
        commands: vk::CommandBuffer,
        
        semaphore: vk::Semaphore,
        swapchain: vk::SwapchainKHR,
        image_index: u32,

        /// Optionally used to schedule swapchain presentation if more than one
        /// is present. The smallest ones go first.
        time_to_next_vsync: Duration,
    }
}

pub struct Renderer {
}

impl Renderer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn submit(&self, command_buffers: &[vk::CommandBuffer], fences: &[vk::Fence]) {
        todo!()
    }

    pub fn present(&self, swapchains: &[vk::SwapchainKHR], images: &[u32], semaphores: &[vk::Semaphore]) {
        todo!()
    }
}
