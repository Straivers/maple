use std::{collections::HashMap, ffi::CStr, rc::Rc};

use crate::constants::{TRIANGLE_FRAGMENT_SHADER, TRIANGLE_VERTEX_SHADER};
use crate::error::Result;
use ash::vk;
use vulkan_utils::Context;

pub trait Effect {
    fn render_pass(&self) -> vk::RenderPass;
    fn apply(&self, context: &Context, target: vk::Framebuffer, target_rect: vk::Rect2D, cmd: vk::CommandBuffer);
}

#[derive(Default)]
pub struct TriangleEffectBase {
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    effects: HashMap<vk::Format, Rc<TriangleEffect>>,
}

impl TriangleEffectBase {
    pub fn new(context: &mut Context) -> Result<Self> {
        let vertex_shader = context.create_shader(TRIANGLE_VERTEX_SHADER)?;
        let fragment_shader = context.create_shader(TRIANGLE_FRAGMENT_SHADER)?;

        let pipeline_layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder();

            unsafe { context.device.create_pipeline_layout(&create_info, None) }?
        };

        Ok(Self {
            vertex_shader,
            fragment_shader,
            pipeline_layout,
            effects: HashMap::new(),
        })
    }

    pub fn destroy(mut this: TriangleEffectBase, context: &mut Context) {
        this.cleanup(context);
        assert!(
            this.effects.is_empty(),
            "Cannot destroy effect base while its derivations are in use!"
        );

        unsafe {
            context.device.destroy_shader_module(this.vertex_shader, None);
            context.device.destroy_shader_module(this.fragment_shader, None);
            context.device.destroy_pipeline_layout(this.pipeline_layout, None);
        }
    }

    pub fn get_effect(&mut self, context: &mut Context, output_format: vk::Format) -> Result<Rc<TriangleEffect>> {
        if let Some(effect) = self.effects.get(&output_format) {
            Ok(effect.clone())
        } else {
            let effect = Rc::new(TriangleEffect::new(self, context, output_format)?);

            self.effects.insert(output_format, effect.clone());

            Ok(effect)
        }
    }

    pub fn cleanup(&mut self, context: &mut Context) {
        self.effects.retain(|_, effect| {
            let keep = Rc::strong_count(effect) > 1;
            if !keep {
                unsafe {
                    context.device.destroy_render_pass(effect.render_pass, None);
                    context.device.destroy_pipeline(effect.pipeline, None);
                }
            }
            keep
        });
    }
}

pub struct TriangleEffect {
    pub format: vk::Format,
    pub render_pass: vk::RenderPass,
    pub pipeline: vk::Pipeline,
}

impl TriangleEffect {
    pub fn new(base: &TriangleEffectBase, context: &mut Context, output_format: vk::Format) -> Result<Self> {
        let render_pass = create_renderpass(context, output_format)?;
        let pipeline = create_pipeline(
            context,
            base.vertex_shader,
            base.fragment_shader,
            render_pass,
            base.pipeline_layout,
        )?;

        Ok(Self {
            format: output_format,
            render_pass,
            pipeline,
        })
    }
}

impl Effect for TriangleEffect {
    fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }

    fn apply(&self, context: &Context, target: vk::Framebuffer, target_rect: vk::Rect2D, cmd: vk::CommandBuffer) {
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

fn create_renderpass(context: &Context, format: vk::Format) -> Result<vk::RenderPass> {
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

    let attachment_reference = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let subpasses = [vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&[attachment_reference])
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

    Ok(unsafe { context.device.create_render_pass(&create_info, None)? })
}

fn create_pipeline(
    context: &mut Context,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
) -> Result<vk::Pipeline> {
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

    Ok(context.create_graphics_pipeline(&create_info)?)
}
