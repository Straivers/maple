use std::rc::Rc;

use ash::vk;

pub trait Effect {
    fn render_pass(&self) -> vk::RenderPass;
    fn apply(
        &self,
        context: &vulkan_utils::Context,
        target: vk::Framebuffer,
        target_rect: vk::Rect2D,
        cmd: vk::CommandBuffer,
    );
}

pub trait EffectBase {
    fn cleanup(&mut self, context: &vulkan_utils::Context);

    fn destroy(self, context: &vulkan_utils::Context);

    fn get_effect(&mut self, context: &vulkan_utils::Context, format: vk::Format) -> Rc<dyn Effect>;
}
