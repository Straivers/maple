// struct or trait?
// trait would allow substitution... do we need that?
// struct at first, transition to trait only if absolutely necessary

use std::{convert::TryInto, process::abort, sync::mpsc::Sender};

use lazy_static::lazy_static;

use ash::vk;
use sys::library::Library;
use vulkan_utils::Vulkan;

lazy_static! {
    static ref VULKAN: Vulkan = {
        let mut verify = cfg!(debug_assertions);
        match std::env::var("MAPLE_CHECK_VULKAN") {
            Ok(val) => {
                match val.parse() {
                    Ok(0) => verify = false,
                    Ok(1) => verify = true,
                    Ok(_) | Err(_) => {
                        println!("MAPLE_CHECK_VULKAN must be absent, or else have a value of 0 or 1");
                        abort();
                    }
                };
            }
            Err(_) => {}
        };

        let library = Library::load("vulkan-1").unwrap();
        println!("verify: {}", verify);
        Vulkan::new(library, verify)
    };
}

#[derive(Debug)]
pub enum RenderResponse {
    CommandsSubmitted,
}

#[derive(Debug)]
pub enum RenderMessage {
    Empty,
    Submit {
        fence: vk::Fence,
        wait_semaphore: vk::Semaphore,
        signal_semaphore: vk::Semaphore,
        commands: vk::CommandBuffer,
        ack: Sender<RenderResponse>,
    },
}

pub struct Renderer {}

impl Renderer {
    pub fn new() -> Self {
        lazy_static::initialize(&VULKAN);
        Self {}
    }

    pub fn submit(
        &self,
        command_buffers: vk::CommandBuffer,
        wait: vk::Semaphore,
        signal: vk::Semaphore,
        fence: vk::Fence,
    ) {
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &wait,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            signal_semaphore_count: 1,
            p_signal_semaphores: &signal,
            command_buffer_count: 1,
            p_command_buffers: &command_buffers,
        };

        VULKAN.submit_to_graphics_queue(&[submit_info], fence);
    }
}
