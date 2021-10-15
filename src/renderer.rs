
// struct or trait?
// trait would allow substitution... do we need that?
    // struct at first, transition to trait only if absolutely necessary

use ash::vk;

pub enum RenderMessage {
    SubmitAndPresent {
        fence: vk::Fence,
        commands: vk::CommandBuffer,
        
        semaphore: vk::Semaphore,
        swapchain: vk::SwapchainKHR,
        image_index: u32,

        time_to_next_vsync: u64,
    }
}

pub struct Renderer {

}
