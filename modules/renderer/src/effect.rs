use ash::vk;
use vulkan_utils::CommandRecorder;

pub trait Effect {
    fn render_pass(&self) -> vk::RenderPass;
    fn apply(
        &self,
        cmd: &CommandRecorder,
        target: vk::Framebuffer,
        layout: vk::PipelineLayout,
        target_rect: vk::Rect2D,
        num_indices: u32,
        vertex_buffer: (vk::Buffer, vk::DeviceSize),
        index_buffer: (vk::Buffer, vk::DeviceSize),
    );
}

pub trait EffectBase {
    fn cleanup(&mut self, context: &vulkan_utils::Context);

    fn destroy(self, context: &vulkan_utils::Context);

    fn get_effect(&mut self, context: &vulkan_utils::Context, format: vk::Format) -> &dyn Effect;
}
