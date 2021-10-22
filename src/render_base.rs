use std::{ffi::CStr, process::abort};

use ash::vk;
use lazy_static::lazy_static;

use renderer::{color::Color, geometry::float2};
use sys::{dpi::PhysicalSize, library::Library};
use vulkan_utils::{CommandRecorder, Vulkan};

use crate::constants::{FRAMES_IN_FLIGHT, TRIANGLE_FRAGMENT_SHADER_SPIRV, TRIANGLE_VERTEX_SHADER_SPIRV};

lazy_static! {
    pub static ref VULKAN: Vulkan = {
        let mut verify = cfg!(debug_assertions);
        if let Ok(val) = std::env::var("MAPLE_CHECK_VULKAN") {
            match val.parse() {
                Ok(0) => verify = false,
                Ok(1) => verify = true,
                Ok(_) | Err(_) => {
                    println!("MAPLE_CHECK_VULKAN must be absent, or else have a value of 0 or 1");
                    abort();
                }
            }
        }

        let library = Library::load("vulkan-1").unwrap();
        Vulkan::new(library, verify)
    };
    pub static ref VERTEX_SHADER: vk::ShaderModule = VULKAN.create_shader(TRIANGLE_VERTEX_SHADER_SPIRV);
    pub static ref FRAGMENT_SHADER: vk::ShaderModule = VULKAN.create_shader(TRIANGLE_FRAGMENT_SHADER_SPIRV);
    pub static ref PIPELINE_LAYOUT: vk::PipelineLayout = {
        let push_constants = [vk::PushConstantRange {
            offset: 0,
            size: std::mem::size_of::<float2>() as u32,
            stage_flags: vk::ShaderStageFlags::VERTEX,
        }];

        let create_info = vk::PipelineLayoutCreateInfo::builder().push_constant_ranges(&push_constants);
        VULKAN.create_pipeline_layout(&create_info)
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: float2,
    pub color: Color,
}

impl Vertex {
    pub const BINDING_DESCRIPTION: vk::VertexInputBindingDescription = vk::VertexInputBindingDescription {
        binding: 0,
        stride: std::mem::size_of::<Vertex>() as u32,
        input_rate: vk::VertexInputRate::VERTEX,
    };

    pub const ATTRIBUTE_DESCRIPTION: [vk::VertexInputAttributeDescription; 2] = [
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: 0,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            format: vk::Format::R8G8B8A8_UNORM,
            offset: std::mem::size_of::<float2>() as u32,
        },
    ];
}

#[must_use]
#[derive(Debug)]
pub enum Request {
    /// Notifies the [Renderer](crate::renderer::Renderer) that a new context
    /// has been created, and requests enough state to initialize a new context.
    ContextInit,
    /// Requests that the [Renderer](crate::renderer::Renderer) submit a command
    /// buffer to the graphics queue for rendering.
    SubmitCommands {
        wait_semaphore: vk::Semaphore,
        signal_semaphore: vk::Semaphore,
        commands: vk::CommandBuffer,
        fence: vk::Fence,
        swapchain: vk::SwapchainKHR,
        image_id: u32,
    },
}

#[must_use]
#[derive(Debug)]
pub enum Response {
    /// The response from the [Renderer](crate::renderer::Renderer) to a window
    /// that submitted a [ContextInitRequest].
    ContextInit {
        fences: [vk::Fence; FRAMES_IN_FLIGHT],
        wait_semaphores: [vk::Semaphore; FRAMES_IN_FLIGHT],
        signal_semaphores: [vk::Semaphore; FRAMES_IN_FLIGHT],
    },
    /// The [Renderer](crate::renderer::Renderer) has submitted the queue for
    /// rendering, and returns a fence that the window thread can use to wait
    /// until rendering is complete.
    CommandsSubmitted { image_id: u32 },
}

pub fn to_extent(size: PhysicalSize) -> vk::Extent2D {
    vk::Extent2D {
        width: size.width.into(),
        height: size.height.into(),
    }
}

pub fn record_command_buffer(
    cmd: &CommandRecorder,
    viewport: vk::Rect2D,
    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    target: vk::Framebuffer,
    vertex_buffer: vk::Buffer,
    vertex_buffer_offset: vk::DeviceSize,
    index_buffer: vk::Buffer,
    index_buffer_offset: vk::DeviceSize,
    num_indices: u32,
) {
    cmd.begin();
    {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        cmd.begin_render_pass(
            &vk::RenderPassBeginInfo::builder()
                .render_pass(render_pass)
                .framebuffer(target)
                .render_area(viewport)
                .clear_values(&clear_values),
            vk::SubpassContents::INLINE,
        );
    }

    cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, pipeline);

    let vertex_buffers = [vertex_buffer];
    let offsets = [vertex_buffer_offset];
    cmd.bind_vertex_buffers(0, &vertex_buffers, &offsets);
    cmd.bind_index_buffer(index_buffer, index_buffer_offset, vk::IndexType::UINT16);

    cmd.set_viewport(&[vk::Viewport {
        x: viewport.offset.x as f32,
        y: viewport.offset.y as f32,
        width: viewport.extent.width as f32,
        height: viewport.extent.height as f32,
        min_depth: 0.0,
        max_depth: 0.0,
    }]);

    cmd.set_scissor(&[viewport]);

    let scale = float2(2.0 / viewport.extent.width as f32, 2.0 / viewport.extent.height as f32);

    cmd.push_constants(layout, vk::ShaderStageFlags::VERTEX, 0, &scale);

    cmd.draw_indexed(num_indices, 1, 0, 0, 0);
    cmd.end_render_pass();
    cmd.end();
}

pub fn create_render_pass(format: vk::Format) -> vk::RenderPass {
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

    VULKAN.create_render_pass(&create_info)
}

pub fn create_pipeline(layout: vk::PipelineLayout, render_pass: vk::RenderPass) -> vk::Pipeline {
    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(*VERTEX_SHADER)
            .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") })
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(*FRAGMENT_SHADER)
            .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") })
            .build(),
    ];

    let vertex_binding_descriptions = [Vertex::BINDING_DESCRIPTION];
    let attribute_binding_descriptions = Vertex::ATTRIBUTE_DESCRIPTION;
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&vertex_binding_descriptions)
        .vertex_attribute_descriptions(&attribute_binding_descriptions);

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
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0);

    VULKAN.create_graphics_pipeline(&create_info)
}
