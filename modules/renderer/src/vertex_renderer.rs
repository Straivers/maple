use std::{collections::HashMap, convert::TryInto, ffi::CStr, rc::Rc};

use crate::effect::{Effect, EffectBase};
use crate::swapchain::Swapchain;
use ash::vk;
use sys::library::Library;

const VERTEX_SHADER: &[u8] = include_bytes!("../shaders/simple_vertex_vert.spv");
const FRAGMENT_SHADER: &[u8] = include_bytes!("../shaders/simple_vertex_frag.spv");

#[derive(Debug, Clone, Copy)]
struct Vec<T: Copy, const SIZE: usize> {
    parts: [T; SIZE],
}

type float2 = Vec<f32, 2>;
type float3 = Vec<f32, 3>;

#[repr(C)]
struct Vertex {
    position: float2,
    color: float3,
}

impl Vertex {
    fn position_offset() -> u32 {
        0
    }

    fn color_offset() -> u32 {
        let previous = std::alloc::Layout::new::<float2>();
        previous.size() as u32
    }

    fn description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>().try_into().unwrap(),
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    fn attributes() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::size_of::<float2>().try_into().unwrap(),
            },
        ]
    }
}

struct SimpleVertexRenderer {
    context: vulkan_utils::Context,
    effect_base: SimpleVertexEffectBase,
}

impl SimpleVertexRenderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Self {
        let context = vulkan_utils::Context::new(vulkan_library, debug_mode);
        let effect_base = SimpleVertexEffectBase::new(&context);

        Self { context, effect_base }
    }

    pub fn create_swapchain(&mut self, window: sys::window::WindowRef) -> Swapchain {
        Swapchain::new(&mut self.context, window, &mut self.effect_base)
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        swapchain.destroy(&mut self.context)
    }

    pub fn end_frame(&mut self) {
        self.effect_base.cleanup(&mut self.context);
    }
}

struct SimpleVertexEffectBase {
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    instances: HashMap<vk::Format, Rc<SimpleVertexEffect>>,
}

impl SimpleVertexEffectBase {
    fn new(context: &vulkan_utils::Context) -> Self {
        let vertex_shader = context.create_shader(VERTEX_SHADER);
        let fragment_shader = context.create_shader(FRAGMENT_SHADER);

        let pipeline_layout = {
            let create_info = vk::PipelineLayoutCreateInfo::default();
            context.create_pipeline_layout(&create_info)
        };

        SimpleVertexEffectBase {
            vertex_shader,
            fragment_shader,
            pipeline_layout,
            instances: HashMap::new(),
        }
    }
}

impl EffectBase for SimpleVertexEffectBase {
    fn cleanup(&mut self, context: &vulkan_utils::Context) {
        self.instances.retain(|_, effect| {
            let keep = Rc::strong_count(effect) > 1;
            if !keep {
                context.destroy_render_pass(effect.renderpass);
                context.destroy_pipeline(effect.pipeline);
            }
            keep
        });
    }

    fn destroy(mut self, context: &vulkan_utils::Context) {
        context.destroy_shader(self.vertex_shader);
        context.destroy_shader(self.fragment_shader);
        context.destroy_pipeline_layout(self.pipeline_layout);
        self.cleanup(context);
        assert!(
            self.instances.is_empty(),
            "All instances of an effect must be destroyed before the effect base"
        );
    }

    fn get_effect(&mut self, context: &vulkan_utils::Context, output_format: vk::Format) -> std::rc::Rc<dyn Effect> {
        if let Some(effect) = self.instances.get(&output_format) {
            effect.clone()
        } else {
            let effect = Rc::new(SimpleVertexEffect::new(self, context, output_format));
            self.instances.insert(output_format, effect.clone());
            effect
        }
    }
}

impl Drop for SimpleVertexEffectBase {
    fn drop(&mut self) {
        assert!(
            self.instances.is_empty(),
            "EffectBase::destroy() must be called before dropping!"
        );
    }
}

struct SimpleVertexEffect {
    renderpass: vk::RenderPass,
    pipeline: vk::Pipeline,
}

impl SimpleVertexEffect {
    fn new(effect_base: &SimpleVertexEffectBase, context: &vulkan_utils::Context, output_format: vk::Format) -> Self {
        let renderpass = create_renderpass(context, output_format);
        let pipeline = create_pipeline(
            context,
            effect_base.vertex_shader,
            effect_base.fragment_shader,
            renderpass,
            effect_base.pipeline_layout,
        );

        Self { renderpass, pipeline }
    }
}

impl Effect for SimpleVertexEffect {
    fn render_pass(&self) -> vk::RenderPass {
        self.renderpass
    }

    fn apply(
        &self,
        context: &vulkan_utils::Context,
        target: vk::Framebuffer,
        target_rect: vk::Rect2D,
        cmd: vk::CommandBuffer,
    ) {
        todo!()
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
    renderpass: vk::RenderPass,
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

    let vertex_binding_descriptions = [Vertex::description()];
    let vertex_binding_attributes = Vertex::attributes();

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&vertex_binding_descriptions[..])
        .vertex_attribute_descriptions(&vertex_binding_attributes);

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
        .render_pass(renderpass)
        .subpass(0);

    context.create_graphics_pipeline(&create_info)
}
