use std::ffi::CStr;

use ash::vk::{self, SampleCountFlags};
use sys::library::Library;

type Result<T> = std::result::Result<T, Error>;

const TRIANGLE_VERTEX_SHADER: &[u8] = include_bytes!("../shaders/tri.vert.spv");
const TRIANGLE_FRAGMENT_SHADER: &[u8] = include_bytes!("../shaders/tri.frag.spv");

struct ShaderSource<const COUNT: usize>([u8; COUNT]);

#[derive(Debug)]
pub enum Error {
    InternalError(Box<dyn std::error::Error>),
}

#[doc(hidden)]
impl From<vulkan_utils::Error> for Error {
    fn from(vkr: vulkan_utils::Error) -> Self {
        Error::InternalError(Box::new(vkr))
    }
}

#[doc(hidden)]
impl From<vk::Result> for Error {
    fn from(vkr: vk::Result) -> Self {
        Error::InternalError(Box::new(vulkan_utils::Error::from(vkr)))
    }
}

pub struct Swapchain {
    swapchain: vulkan_utils::Swapchain,
    window: sys::window::WindowRef,
    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
}

pub struct TriangleRenderer {
    vulkan: vulkan_utils::Context,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
}

impl TriangleRenderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Result<Self> {
        let mut vulkan = vulkan_utils::Context::new(vulkan_library, debug_mode)?;

        let vertex_shader = vulkan.create_shader(TRIANGLE_VERTEX_SHADER)?;
        let fragment_shader = vulkan.create_shader(TRIANGLE_FRAGMENT_SHADER)?;

        let pipeline_layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder();

            unsafe { vulkan.device.create_pipeline_layout(&create_info, None) }?
        };

        Ok(Self {
            vulkan,
            vertex_shader,
            fragment_shader,
            pipeline_layout,
        })
    }

    pub fn create_swapchain(&mut self, window: sys::window::WindowRef) -> Result<Swapchain> {
        let swapchain = vulkan_utils::Swapchain::new(&mut self.vulkan, &window)?;
        let render_pass = create_renderpass(&self.vulkan, swapchain.format)?;
        let pipeline = create_pipeline(&mut self.vulkan, self.vertex_shader, self.fragment_shader, render_pass, self.pipeline_layout)?;

        Ok(Swapchain {
            swapchain,
            window,
            render_pass,
            pipeline,
        })
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        swapchain.swapchain.destroy(&mut self.vulkan);
        unsafe {
            self.vulkan.device.destroy_pipeline(swapchain.pipeline, None);
            self.vulkan.device.destroy_render_pass(swapchain.render_pass, None);
        }
    }

    pub fn render_to(&mut self, swapchain: &mut Swapchain) {}
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {
        self.vulkan.destroy_shader(self.vertex_shader);
        self.vulkan.destroy_shader(self.fragment_shader);
        unsafe { self.vulkan.device.destroy_pipeline_layout(self.pipeline_layout, None); }
    }
}

fn create_renderpass(context: &vulkan_utils::Context, format: vk::Format) -> Result<vk::RenderPass> {
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

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses);

    Ok(unsafe { context.device.create_render_pass(&create_info, None)? })
}

fn create_pipeline(
    context: &mut vulkan_utils::Context,
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
        .rasterization_samples(SampleCountFlags::TYPE_1);

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
