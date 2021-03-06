use std::{ffi::CStr, process::abort};

use ash::vk::{self, DependencyFlags};
use lazy_static::lazy_static;

use super::{color::Color, recorder::Recorder, vulkan::Vulkan};
use crate::{shapes::Extent, sys::Library};

pub const TRIANGLE_VERTEX_SHADER_SPIRV: &[u8] =
    include_bytes!("../../shaders/simple_vertex_vert.spv");
pub const TRIANGLE_FRAGMENT_SHADER_SPIRV: &[u8] =
    include_bytes!("../../shaders/simple_vertex_frag.spv");

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
    pub static ref VERTEX_SHADER: vk::ShaderModule =
        VULKAN.create_shader(TRIANGLE_VERTEX_SHADER_SPIRV);
    pub static ref FRAGMENT_SHADER: vk::ShaderModule =
        VULKAN.create_shader(TRIANGLE_FRAGMENT_SHADER_SPIRV);
    pub static ref PIPELINE_LAYOUT: vk::PipelineLayout = {
        let push_constants = [vk::PushConstantRange {
            offset: 0,
            size: std::mem::size_of::<Scale>() as u32,
            stage_flags: vk::ShaderStageFlags::VERTEX,
        }];

        let create_info =
            vk::PipelineLayoutCreateInfo::builder().push_constant_ranges(&push_constants);
        VULKAN.create_pipeline_layout(&create_info)
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: (f32, f32),
    pub color: Color,
}

pub struct Scale {
    #[allow(dead_code)]
    // Read by shader, so it's ok if this variable isn't read on the CPU
    horizontal: f32,
    #[allow(dead_code)]
    // Read by shader, so it's ok if this variable isn't read on the CPU
    vertical: f32,
}

impl Vertex {
    pub const BINDING_DESCRIPTION: vk::VertexInputBindingDescription =
        vk::VertexInputBindingDescription {
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
            offset: std::mem::size_of::<(f32, f32)>() as u32,
        },
    ];
}

#[must_use]
#[derive(Debug)]
pub enum Request {
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
    /// The [Renderer](crate::renderer::Renderer) has submitted the queue for
    /// rendering, and returns a fence that the window thread can use to wait
    /// until rendering is complete.
    CommandsSubmitted { image_id: u32 },
}

pub fn to_extent(size: Extent) -> vk::Extent2D {
    vk::Extent2D {
        width: size.width.0 as u32,
        height: size.height.0 as u32,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn record_command_buffer(
    cmd: &Recorder,
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

    let scale = Scale {
        vertical: 2.0 / viewport.extent.height as f32,
        horizontal: 2.0 / viewport.extent.width as f32,
    };
    cmd.push_constants(layout, vk::ShaderStageFlags::VERTEX, 0, &scale);

    cmd.draw_indexed(num_indices, 1, 0, 0, 0);
    cmd.end_render_pass();
    cmd.end();
}

pub fn create_render_pass(format: vk::Format) -> vk::RenderPass {
    let attachments = [vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
    }];

    let attachment_reference = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];

    let subpasses = [vk::SubpassDescription {
        pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
        p_color_attachments: attachment_reference.as_ptr(),
        color_attachment_count: attachment_reference.len() as u32,
        ..Default::default()
    }];

    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        dst_subpass: 0,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        src_access_mask: vk::AccessFlags::empty(),
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dependency_flags: DependencyFlags::empty(),
    }];

    let create_info = vk::RenderPassCreateInfo {
        p_attachments: attachments.as_ptr(),
        attachment_count: 1,
        p_subpasses: subpasses.as_ptr(),
        subpass_count: 1,
        p_dependencies: dependencies.as_ptr(),
        dependency_count: 1,
        ..Default::default()
    };

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
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
        p_vertex_binding_descriptions: vertex_binding_descriptions.as_ptr(),
        vertex_binding_description_count: vertex_binding_descriptions.len() as u32,
        p_vertex_attribute_descriptions: attribute_binding_descriptions.as_ptr(),
        vertex_attribute_description_count: attribute_binding_descriptions.len() as u32,
        ..Default::default()
    };

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        primitive_restart_enable: vk::FALSE,
        ..Default::default()
    };

    let viewport_state = vk::PipelineViewportStateCreateInfo {
        viewport_count: 1,
        scissor_count: 1,
        ..Default::default()
    };

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
        depth_clamp_enable: vk::FALSE,
        rasterizer_discard_enable: vk::FALSE,
        polygon_mode: vk::PolygonMode::FILL,
        line_width: 1.0,
        cull_mode: vk::CullModeFlags::BACK,
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        depth_bias_enable: vk::FALSE,
        ..Default::default()
    };

    let multisample_state = vk::PipelineMultisampleStateCreateInfo {
        sample_shading_enable: vk::FALSE,
        rasterization_samples: vk::SampleCountFlags::TYPE_1,
        ..Default::default()
    };

    let color_blend_attachments = [vk::PipelineColorBlendAttachmentState {
        color_write_mask: vk::ColorComponentFlags::R
            | vk::ColorComponentFlags::G
            | vk::ColorComponentFlags::B
            | vk::ColorComponentFlags::A,
        blend_enable: vk::FALSE,
        ..Default::default()
    }];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
        logic_op_enable: vk::FALSE,
        p_attachments: color_blend_attachments.as_ptr(),
        attachment_count: color_blend_attachments.len() as u32,
        ..Default::default()
    };

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_state = vk::PipelineDynamicStateCreateInfo {
        p_dynamic_states: dynamic_states.as_ptr(),
        dynamic_state_count: dynamic_states.len() as u32,
        ..Default::default()
    };

    let create_info = vk::GraphicsPipelineCreateInfo {
        p_stages: shader_stages.as_ptr(),
        stage_count: shader_stages.len() as u32,
        p_vertex_input_state: &vertex_input_state,
        p_input_assembly_state: &input_assembly_state,
        p_viewport_state: &viewport_state,
        p_rasterization_state: &rasterization_state,
        p_multisample_state: &multisample_state,
        p_color_blend_state: &color_blend_state,
        p_dynamic_state: &dynamic_state,
        layout: layout,
        render_pass: render_pass,
        subpass: 0,
        ..Default::default()
    };

    VULKAN.create_graphics_pipeline(&create_info)
}
