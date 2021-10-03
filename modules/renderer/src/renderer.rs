use std::convert::TryInto;
use std::{collections::HashMap, ffi::CStr};

use ash::vk;
use sys::library::Library;
use sys::{dpi::PhysicalSize, window_handle::WindowHandle};

use vulkan_utils::CommandRecorder;

use crate::effect::{Effect, EffectBase};
use crate::vertex::Vertex;
use crate::window_context::{physical_size_to_extent, WindowContext};

pub const TRIANGLE_VERTEX_SHADER: &[u8] = include_bytes!("../shaders/simple_vertex_vert.spv");
pub const TRIANGLE_FRAGMENT_SHADER: &[u8] = include_bytes!("../shaders/simple_vertex_frag.spv");

pub struct Renderer {
    vulkan: vulkan_utils::Context,
    effect_base: RenderEffectBase,
}

impl Renderer {
    pub fn new(vulkan_library: Library, debug_mode: bool) -> Self {
        let mut vulkan = vulkan_utils::Context::new(vulkan_library, debug_mode);
        let effect_base = RenderEffectBase::new(&mut vulkan);

        Self { vulkan, effect_base }
    }

    pub fn create_swapchain(
        &mut self,
        window_handle: WindowHandle,
        framebuffer_size: PhysicalSize,
    ) -> WindowContext<Vertex> {
        WindowContext::new(&mut self.vulkan, window_handle, framebuffer_size, &mut self.effect_base)
    }

    pub fn destroy_swapchain(&mut self, swapchain: WindowContext<Vertex>) {
        swapchain.destroy(&mut self.vulkan)
    }

    pub fn end_frame(&mut self) {
        self.effect_base.cleanup(&self.vulkan);
    }

    pub fn render_to(
        &mut self,
        swapchain: &mut WindowContext<Vertex>,
        target_size: PhysicalSize,
        vertices: &[Vertex],
        indices: &[u16],
    ) {
        if target_size == (PhysicalSize { width: 0, height: 0 }) {
            return;
        }

        let target_extent = physical_size_to_extent(target_size);

        let (frame, frame_sync) = swapchain
            .next_frame(&mut self.vulkan, target_size, &mut self.effect_base)
            .unwrap();

        frame.copy_data_to_gpu(&mut self.vulkan, vertices, indices);

        let viewport_rect = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: target_extent,
        };

        let cmd = self.vulkan.record_command_buffer(frame.command_buffer);

        cmd.begin();

        self.effect_base.get_effect(&self.vulkan, frame.image_format).apply(
            &cmd,
            frame.frame_buffer,
            viewport_rect,
            indices.len().try_into().expect("Number of vertices exceeds u32::MAX"),
            frame.vertex_buffer(),
            frame.index_buffer(),
        );

        cmd.end();

        {
            let submit_info = vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: std::ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: &frame_sync.acquire_semaphore,
                p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                signal_semaphore_count: 1,
                p_signal_semaphores: &frame_sync.present_semaphore,
                command_buffer_count: 1,
                p_command_buffers: &cmd.buffer,
            };

            self.vulkan.reset_fences(&[frame_sync.fence]);
            self.vulkan.submit_to_graphics_queue(&[submit_info], frame_sync.fence);
        }

        swapchain.present(&mut self.vulkan);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        RenderEffectBase::destroy(std::mem::take(&mut self.effect_base), &self.vulkan);
    }
}

#[derive(Default)]
struct RenderEffectBase {
    generation: u64,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    effects: HashMap<vk::Format, RenderEffect>,
}

impl RenderEffectBase {
    fn new(context: &mut vulkan_utils::Context) -> Self {
        let vertex_shader = context.create_shader(TRIANGLE_VERTEX_SHADER);
        let fragment_shader = context.create_shader(TRIANGLE_FRAGMENT_SHADER);

        let pipeline_layout = {
            let create_info = vk::PipelineLayoutCreateInfo::builder();
            context.create_pipeline_layout(&create_info)
        };

        Self {
            generation: 0,
            vertex_shader,
            fragment_shader,
            pipeline_layout,
            effects: HashMap::new(),
        }
    }
}

impl EffectBase for RenderEffectBase {
    fn cleanup(&mut self, context: &vulkan_utils::Context) {
        self.generation += 1;

        let generation = self.generation;
        self.effects.retain(|_, effect| {
            let keep = effect.generation + 2 >= generation;
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

    fn get_effect(&mut self, context: &vulkan_utils::Context, output_format: vk::Format) -> &dyn Effect {
        // These are copied out so that `self` doesn't have to be borrowed in
        // `or_insert_with()`
        let generation = self.generation;
        let vertex_shader = self.vertex_shader;
        let fragment_shader = self.fragment_shader;
        let pipeline_layout = self.pipeline_layout;

        let entry = self.effects.entry(output_format).or_insert_with(|| {
            let render_pass = create_renderpass(context, output_format);
            let pipeline = create_pipeline(context, vertex_shader, fragment_shader, render_pass, pipeline_layout);

            RenderEffect {
                render_pass,
                pipeline,
                generation,
            }
        });

        entry.generation = self.generation;
        entry
    }
}

struct RenderEffect {
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    generation: u64,
}

impl Effect for RenderEffect {
    fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }

    fn apply(
        &self,
        cmd: &CommandRecorder,
        target: vk::Framebuffer,
        target_rect: vk::Rect2D,
        num_indices: u32,
        vertex_buffer: (vk::Buffer, vk::DeviceSize),
        index_buffer: (vk::Buffer, vk::DeviceSize),
    ) {
        {
            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            cmd.begin_render_pass(
                &vk::RenderPassBeginInfo::builder()
                    .render_pass(self.render_pass)
                    .framebuffer(target)
                    .render_area(target_rect)
                    .clear_values(&clear_values),
                vk::SubpassContents::INLINE,
            );
        }

        cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline);

        let vertex_buffers = [vertex_buffer.0];
        let offsets = [vertex_buffer.1];
        cmd.bind_vertex_buffers(0, &vertex_buffers, &offsets);
        cmd.bind_index_buffer(index_buffer.0, index_buffer.1, vk::IndexType::UINT16);

        cmd.set_viewport(&[vk::Viewport {
            x: target_rect.offset.x as f32,
            y: target_rect.offset.y as f32,
            width: target_rect.extent.width as f32,
            height: target_rect.extent.height as f32,
            min_depth: 0.0,
            max_depth: 0.0,
        }]);

        cmd.set_scissor(&[target_rect]);
        cmd.draw_indexed(num_indices, 1, 0, 0, 0);
        cmd.end_render_pass();
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
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0);

    context.create_graphics_pipeline(&create_info)
}
