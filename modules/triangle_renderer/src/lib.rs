use std::ffi::CStr;

use ash::vk;
use sys::library::Library;

type Result<T> = std::result::Result<T, Error>;

const FRAMES_IN_FLIGHT: usize = 2;
const TRIANGLE_VERTEX_SHADER: &[u8] = include_bytes!("../shaders/tri.vert.spv");
const TRIANGLE_FRAGMENT_SHADER: &[u8] = include_bytes!("../shaders/tri.frag.spv");

#[derive(Debug)]
pub enum Error {
    WindowNotValid,
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
    current_frame: usize,
    swapchain: vulkan_utils::Swapchain,
    window: sys::window::WindowRef,
    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    sync_acquire: [vk::Semaphore; FRAMES_IN_FLIGHT],
    sync_present: [vk::Semaphore; FRAMES_IN_FLIGHT],
    sync_fence: [vk::Fence; FRAMES_IN_FLIGHT],
    running_commands: [Vec<vk::CommandBuffer>; FRAMES_IN_FLIGHT],
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
        let swapchain = vulkan_utils::Swapchain::new(&mut self.vulkan, &window, (FRAMES_IN_FLIGHT + 1) as u32)?;
        let render_pass = create_renderpass(&self.vulkan, swapchain.format)?;
        let pipeline = create_pipeline(
            &mut self.vulkan,
            self.vertex_shader,
            self.fragment_shader,
            render_pass,
            self.pipeline_layout,
        )?;

        let framebuffers = {
            let mut buffers = Vec::with_capacity(swapchain.images.len());
            let image_size = window.framebuffer_size().unwrap();

            for image in &swapchain.images {
                let attachments = [image.view];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(image_size.width.into())
                    .height(image_size.height.into())
                    .layers(1);

                buffers.push(unsafe { self.vulkan.device.create_framebuffer(&create_info, None) }?);
            }
            buffers
        };

        Ok(Swapchain {
            current_frame: 0,
            swapchain,
            window,
            render_pass,
            pipeline,
            framebuffers,
            sync_acquire: [
                self.vulkan.get_or_create_semaphore()?,
                self.vulkan.get_or_create_semaphore()?,
            ],
            sync_present: [
                self.vulkan.get_or_create_semaphore()?,
                self.vulkan.get_or_create_semaphore()?,
            ],
            sync_fence: [
                self.vulkan.get_or_create_fence(true)?,
                self.vulkan.get_or_create_fence(true)?,
            ],
            running_commands: [Vec::new(), Vec::new()],
        })
    }

    pub fn destroy_swapchain(&mut self, swapchain: Swapchain) {
        unsafe {
            self.vulkan
                .device
                .wait_for_fences(&swapchain.sync_fence, true, u64::MAX)
                .unwrap();
        }

        swapchain.swapchain.destroy(&mut self.vulkan);
        unsafe {
            for framebuffer in swapchain.framebuffers {
                self.vulkan.device.destroy_framebuffer(framebuffer, None);
            }

            for i in 0..FRAMES_IN_FLIGHT {
                self.vulkan.free_semaphore(swapchain.sync_acquire[i]);
                self.vulkan.free_semaphore(swapchain.sync_present[i]);
                self.vulkan.free_fence(swapchain.sync_fence[i]);
            }

            self.vulkan.device.destroy_pipeline(swapchain.pipeline, None);
            self.vulkan.device.destroy_render_pass(swapchain.render_pass, None);
        }
    }

    pub fn render_to(&mut self, swapchain: &mut Swapchain) -> Result<()> {
        let framebuffer_size = {
            if let Some(size) = swapchain.window.framebuffer_size() {
                size
            } else {
                return Err(Error::WindowNotValid);
            }
        };

        let framebuffer_extent = vk::Extent2D {
            width: framebuffer_size.width.into(),
            height: framebuffer_size.height.into(),
        };

        if swapchain.swapchain.image_size != framebuffer_extent {
            unsafe {
                self.vulkan
                    .device
                    .wait_for_fences(&swapchain.sync_fence, true, u64::MAX)?;
                self.vulkan.device.reset_fences(&swapchain.sync_fence)?;
            }
            // swapchain.resize(self.vulkan, framebuffer_extent);
            todo!()
        }

        let acquire_semaphore = swapchain.sync_acquire[swapchain.current_frame];
        let present_semaphore = swapchain.sync_present[swapchain.current_frame];
        let fence = swapchain.sync_fence[swapchain.current_frame];

        unsafe {
            self.vulkan.device.wait_for_fences(&[fence], true, u64::MAX)?;
            self.vulkan.device.reset_fences(&[fence])?;
        }

        let image_index = {
            let (index, update) = swapchain.swapchain.get_image(&self.vulkan, acquire_semaphore)?;

            if update {
                // swapchain.resize(self.vulkan, framebuffer_extent)
                // let (index2, update2) = swapchain.swapchain.get_image(&self.vulkan, acquire_semaphore)?;
                // assert!(!update2, "vkAcquireNextImage() required resizing twice in a row.");
                // index2
                0
            } else {
                index
            }
        };

        unsafe {
            let old = &mut swapchain.running_commands[swapchain.current_frame];
            if !old.is_empty() {
                self.vulkan
                    .device
                    .free_command_buffers(self.vulkan.graphics_command_pool, old);
                old.clear();
            }
        }

        let command_buffer = {
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.vulkan.graphics_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();

            let mut buffer = vk::CommandBuffer::default();
            unsafe {
                self.vulkan.device.fp_v1_0().allocate_command_buffers(
                    self.vulkan.device.handle(),
                    &alloc_info,
                    &mut buffer,
                )
            }
            .result()?;
            buffer
        };

        let viewport_rect = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: framebuffer_size.width.into(),
                height: framebuffer_size.height.into(),
            },
        };

        {
            let begin_info = vk::CommandBufferBeginInfo::default();
            unsafe { self.vulkan.device.begin_command_buffer(command_buffer, &begin_info) }?
        }

        {
            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            let render_pass_info = vk::RenderPassBeginInfo::builder()
                .render_pass(swapchain.render_pass)
                .framebuffer(swapchain.framebuffers[image_index as usize])
                .render_area(viewport_rect)
                .clear_values(&clear_values);

            unsafe {
                self.vulkan
                    .device
                    .cmd_begin_render_pass(command_buffer, &render_pass_info, vk::SubpassContents::INLINE)
            };
        }

        unsafe {
            self.vulkan
                .device
                .cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, swapchain.pipeline);
        }

        {
            let viewport = vk::Viewport {
                x: viewport_rect.offset.x as f32,
                y: viewport_rect.offset.y as f32,
                width: viewport_rect.extent.width as f32,
                height: viewport_rect.extent.height as f32,
                min_depth: 0.0,
                max_depth: 0.0,
            };

            unsafe { self.vulkan.device.cmd_set_viewport(command_buffer, 0, &[viewport]) }
        }

        unsafe {
            self.vulkan.device.cmd_set_scissor(command_buffer, 0, &[viewport_rect]);
            self.vulkan.device.cmd_draw(command_buffer, 3, 1, 0, 0);
            self.vulkan.device.cmd_end_render_pass(command_buffer);
            self.vulkan.device.end_command_buffer(command_buffer)?;
        }

        {
            let wait = [acquire_semaphore];
            let signal = [present_semaphore];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let commands = [command_buffer];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&wait)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&commands)
                .signal_semaphores(&signal)
                .build();

            unsafe {
                self.vulkan
                    .device
                    .queue_submit(self.vulkan.graphics_queue, &[submit_info], fence)?;
            }
        }

        {
            let wait = [present_semaphore];
            let swapchains = [swapchain.swapchain.handle];
            let image_indices = [image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&wait)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            unsafe {
                self.vulkan
                    .swapchain_api
                    .queue_present(self.vulkan.graphics_queue, &present_info)?
            };
        }

        swapchain.running_commands[swapchain.current_frame].push(command_buffer);
        swapchain.current_frame = (swapchain.current_frame + 1) % FRAMES_IN_FLIGHT;
        Ok(())
    }
}

impl Drop for TriangleRenderer {
    fn drop(&mut self) {
        self.vulkan.destroy_shader(self.vertex_shader);
        self.vulkan.destroy_shader(self.fragment_shader);
        unsafe {
            self.vulkan.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
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
