use ash::vk;
use std::{collections::HashMap, ffi::CStr, rc::Rc};
use sys::dpi::PhysicalSize;
use sys::library::Library;

use crate::constants::FRAMES_IN_FLIGHT;
use crate::effect::{Effect, EffectBase};
use crate::swapchain::Swapchain;

pub const TRIANGLE_VERTEX_SHADER: &[u8] = include_bytes!("../shaders/simple_tri_vert.spv");
pub const TRIANGLE_FRAGMENT_SHADER: &[u8] = include_bytes!("../shaders/simple_tri_frag.spv");

pub struct TriangleRenderer {
    vulkan: vulkan_utils::Context,
    effect_base: TriangleEffectBase,
}

impl TriangleRenderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Self {
        let mut vulkan = vulkan_utils::Context::new(vulkan_library, debug_mode);
        let effect_base = TriangleEffectBase::new(&mut vulkan);

        Self { vulkan, effect_base }
    }

    pub fn create_swapchain<WindowUserData>(&mut self, window: sys::window::WindowRef<WindowUserData>) -> Swapchain {
        Swapchain::new(&mut self.vulkan, window, &mut self.effect_base)
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        swapchain.destroy(&mut self.vulkan)
    }

    pub fn end_frame(&mut self) {
        self.effect_base.cleanup(&self.vulkan);
    }

    pub fn render_to(&mut self, swapchain: &mut Swapchain, target_size: PhysicalSize) {
        let frame = swapchain.frame_in_flight(target_size);

        if frame.extent == vk::Extent2D::default() {
            return;
        }

        if frame.was_resized {
            swapchain.resize(target_size, &mut self.vulkan, &mut self.effect_base);
            return;
        }

        let _ = self.vulkan.wait_for_fences(&[frame.submit_fence], u64::MAX);

        let image_index = if let Some(index) = swapchain.swapchain.get_image(&self.vulkan, frame.acquire_semaphore) {
            index
        } else {
            swapchain.resize(target_size, &mut self.vulkan, &mut self.effect_base);
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
            swapchain.resize(target_size, &mut self.vulkan, &mut self.effect_base);
        }

        swapchain.current_frame = (swapchain.current_frame + 1) % FRAMES_IN_FLIGHT;
    }
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {
        TriangleEffectBase::destroy(std::mem::take(&mut self.effect_base), &self.vulkan);
    }
}

#[derive(Default)]
struct TriangleEffectBase {
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    effects: HashMap<vk::Format, Rc<TriangleEffect>>,
}

impl TriangleEffectBase {
    fn new(context: &mut vulkan_utils::Context) -> Self {
        let vertex_shader = context.create_shader(TRIANGLE_VERTEX_SHADER);
        let fragment_shader = context.create_shader(TRIANGLE_FRAGMENT_SHADER);

        let pipeline_layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder();
            context.create_pipeline_layout(&create_info)
        };

        Self {
            vertex_shader,
            fragment_shader,
            pipeline_layout,
            effects: HashMap::new(),
        }
    }
}

impl EffectBase for TriangleEffectBase {
    fn cleanup(&mut self, context: &vulkan_utils::Context) {
        self.effects.retain(|_, effect| {
            let keep = Rc::strong_count(effect) > 1;
            if !keep {
                context.destroy_render_pass(effect.render_pass);
                context.destroy_pipeline(effect.pipeline);
            }
            keep
        });
    }

    fn destroy(mut self, context: &vulkan_utils::Context) {
        self.cleanup(context);
        assert!(
            self.effects.is_empty(),
            "Cannot destroy effect base while its derivations are in use!"
        );

        context.destroy_shader(self.vertex_shader);
        context.destroy_shader(self.fragment_shader);
        context.destroy_pipeline_layout(self.pipeline_layout);
    }

    fn get_effect(&mut self, context: &vulkan_utils::Context, output_format: vk::Format) -> Rc<dyn Effect> {
        if let Some(effect) = self.effects.get(&output_format) {
            effect.clone()
        } else {
            let effect = Rc::new(TriangleEffect::new(self, context, output_format));
            self.effects.insert(output_format, effect.clone());
            effect
        }
    }
}

struct TriangleEffect {
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
}

impl TriangleEffect {
    fn new(base: &TriangleEffectBase, context: &vulkan_utils::Context, output_format: vk::Format) -> Self {
        let render_pass = create_renderpass(context, output_format);
        let pipeline = create_pipeline(
            context,
            base.vertex_shader,
            base.fragment_shader,
            render_pass,
            base.pipeline_layout,
        );

        Self { render_pass, pipeline }
    }
}

impl Effect for TriangleEffect {
    fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }

    fn apply(
        &self,
        context: &vulkan_utils::Context,
        target: vk::Framebuffer,
        target_rect: vk::Rect2D,
        cmd: vk::CommandBuffer,
    ) {
        {
            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            let render_pass_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(target)
                .render_area(target_rect)
                .clear_values(&clear_values);

            unsafe {
                context
                    .device
                    .cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE)
            };
        }

        unsafe {
            context
                .device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
        }

        {
            let viewport = vk::Viewport {
                x: target_rect.offset.x as f32,
                y: target_rect.offset.y as f32,
                width: target_rect.extent.width as f32,
                height: target_rect.extent.height as f32,
                min_depth: 0.0,
                max_depth: 0.0,
            };

            unsafe { context.device.cmd_set_viewport(cmd, 0, &[viewport]) }
        }

        unsafe {
            context.device.cmd_set_scissor(cmd, 0, &[target_rect]);
            context.device.cmd_draw(cmd, 3, 1, 0, 0);
            context.device.cmd_end_render_pass(cmd);
        }
    }
}

fn create_renderpass(context: &vulkan_utils::Context, format: vk::Format) -> vk::RenderPass {
    let attachments = [vk::AttachmentDescription::builder()
        .format(format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .build()];

    let attachment_reference = [vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build()];

    let subpasses = [vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&attachment_reference)
        .build()];

    let dependencies = [vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .build()];

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    context.create_render_pass(&create_info)
}

fn create_pipeline(
    context: &vulkan_utils::Context,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
) -> vk::Pipeline {
    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader)
            .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") })
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader)
            .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") })
            .build(),
    ];

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder();

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewport_count(1)
        .scissor_count(1);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .blend_enable(false)
        .build()];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

    let create_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&color_blend_state)
        .dynamic_state(&dynamic_state)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0);

    context.create_graphics_pipeline(&create_info)
}
