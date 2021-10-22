///! Types and functions for rendering on a per-window basis.
use ash::vk;
use sys::{dpi::PhysicalSize, window_handle::WindowHandle};

use vulkan_utils::SwapchainData;

use crate::{
    constants::{DEFAULT_VERTEX_BUFFER_SIZE, FRAMES_IN_FLIGHT},
    render_base::{
        create_pipeline, create_render_pass, record_command_buffer, to_extent, Request, Response, Vertex,
        PIPELINE_LAYOUT, VULKAN,
    },
};

pub struct SwapchainImage {
    view: vk::ImageView,
    frame_buffer: vk::Framebuffer,
}

impl Drop for SwapchainImage {
    fn drop(&mut self) {
        VULKAN.destroy_frame_buffer(self.frame_buffer);
        VULKAN.destroy_image_view(self.view);
    }
}

#[derive(Default)]
pub struct Frame {
    fence: vk::Fence,
    acquire: vk::Semaphore,
    present: vk::Semaphore,
    command_buffer: vk::CommandBuffer,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    buffer_size: vk::DeviceSize,
}

/// A [WindowContext] contains all render state needed for a window to
/// communicate with the renderer.
#[derive(Default)]
pub struct RenderContext {
    surface: vk::SurfaceKHR,
    swapchain: SwapchainData,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    command_pool: vk::CommandPool,
    images: Vec<SwapchainImage>,
    frames: [Frame; FRAMES_IN_FLIGHT],
    frame_id: u8,
}

impl RenderContext {
    pub fn new() -> Self {
        let command_pool = VULKAN.create_graphics_command_pool(true, true);
        let mut command_buffers = [vk::CommandBuffer::null(), vk::CommandBuffer::null()];
        VULKAN.allocate_command_buffers(command_pool, &mut command_buffers);

        Self {
            surface: vk::SurfaceKHR::null(),
            swapchain: SwapchainData::default(),
            render_pass: vk::RenderPass::null(),
            pipeline: vk::Pipeline::null(),
            command_pool,
            images: vec![],
            frames: [
                Frame {
                    fence: VULKAN.create_fence(true),
                    acquire: VULKAN.create_semaphore(),
                    present: VULKAN.create_semaphore(),
                    command_buffer: command_buffers[0],
                    buffer: vk::Buffer::null(),
                    memory: vk::DeviceMemory::null(),
                    buffer_size: 0,
                },
                Frame {
                    fence: VULKAN.create_fence(true),
                    acquire: VULKAN.create_semaphore(),
                    present: VULKAN.create_semaphore(),
                    command_buffer: command_buffers[1],
                    buffer: vk::Buffer::null(),
                    memory: vk::DeviceMemory::null(),
                    buffer_size: 0,
                },
            ],
            frame_id: 0,
        }
    }

    pub fn bind(&mut self, window: WindowHandle, window_size: PhysicalSize) {
        let extent = to_extent(window_size);
        let surface = VULKAN.create_surface(window);
        let swapchain = VULKAN.create_swapchain(surface, extent);

        let render_pass = create_render_pass(swapchain.format);
        let pipeline = create_pipeline(*PIPELINE_LAYOUT, render_pass);

        let mut images = vec![];
        Self::init_images(
            &swapchain.images,
            swapchain.format,
            swapchain.image_size,
            render_pass,
            &mut images,
        );

        self.surface = surface;
        self.swapchain = swapchain;
        self.render_pass = render_pass;
        self.pipeline = pipeline;
        self.images = images;
    }

    pub fn draw(&mut self, window_extent: vk::Extent2D, vertices: &[Vertex], indices: &[u16]) -> Option<Request> {
        let frame_id = self.frame_id as usize;
        let frame = &mut self.frames[frame_id];
        let _ = VULKAN.wait_for_fences(&[frame.fence], u64::MAX);

        if window_extent != self.swapchain.image_size {
            self.resize(window_extent);
            return None;
        }

        let acquire_semaphore = frame.acquire;

        let image_index = if let Some(index) = VULKAN.get_swapchain_image(&self.swapchain, acquire_semaphore) {
            index as usize
        } else {
            self.resize(window_extent);
            return None;
        };

        let image = &self.images[image_index];
        VULKAN.reset_command_buffer(frame.command_buffer, false);

        // PERFORMANCE(David Z): It might be more efficient to write verticies
        // and indices directly to mapped memory, especially on integrated GPUs.
        // You'd need the GPU version of a dynamic array though, and I have _no_
        // idea how performant that might be.
        let index_buffer_offset = Self::copy_data_to_gpu(frame, vertices, indices);

        let viewport = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: window_extent,
        };

        // record command buffer
        let cmd = VULKAN.record_command_buffer(frame.command_buffer);
        record_command_buffer(
            &cmd,
            viewport,
            self.pipeline,
            self.render_pass,
            *PIPELINE_LAYOUT,
            image.frame_buffer,
            frame.buffer,
            0,
            frame.buffer,
            index_buffer_offset,
            indices.len() as u32,
        );

        Some(Request::SubmitCommands {
            wait_semaphore: frame.acquire,
            signal_semaphore: frame.present,
            commands: cmd.buffer,
            fence: frame.fence,
            swapchain: self.swapchain.handle,
            image_id: image_index as u32,
        })
    }

    pub fn present(&mut self, response: &Response) {
        if let Response::CommandsSubmitted { image_id } = response {
            VULKAN.present_swapchain_image(
                &self.swapchain,
                &[self.frames[self.frame_id as usize].present],
                *image_id,
            );

            self.frame_id = (self.frame_id + 1) % (FRAMES_IN_FLIGHT as u8);
        } else {
            unreachable!("Queue submission produced unexpected response: {:?}", response);
        }
    }

    fn resize(&mut self, window_extent: vk::Extent2D) {
        let fences = [self.frames[0].fence, self.frames[1].fence];
        let _ = VULKAN.wait_for_fences(&fences, u64::MAX);

        let old_format = self.swapchain.format;
        let old = Some((self.swapchain.handle, std::mem::take(&mut self.swapchain.images)));
        self.swapchain = VULKAN.resize_swapchain(self.surface, window_extent, old);

        if old_format != self.swapchain.format {
            VULKAN.destroy_pipeline(self.pipeline);
            VULKAN.destroy_render_pass(self.render_pass);

            self.render_pass = create_render_pass(self.swapchain.format);
            self.pipeline = create_pipeline(*PIPELINE_LAYOUT, self.render_pass);
        }

        self.images.clear();

        Self::init_images(
            &self.swapchain.images,
            self.swapchain.format,
            self.swapchain.image_size,
            self.render_pass,
            &mut self.images,
        )
    }

    fn init_images(
        images: &[vk::Image],
        format: vk::Format,
        size: vk::Extent2D,
        render_pass: vk::RenderPass,
        result: &mut Vec<SwapchainImage>,
    ) {
        result.reserve(images.len());
        for image in images {
            result.push({
                let view = {
                    let create_info = vk::ImageViewCreateInfo::builder()
                        .image(*image)
                        .format(format)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        });

                    VULKAN.create_image_view(&create_info)
                };

                let frame_buffer = {
                    let attachment = [view];
                    let create_info = vk::FramebufferCreateInfo::builder()
                        .render_pass(render_pass)
                        .attachments(&attachment)
                        .width(size.width)
                        .height(size.height)
                        .layers(1);

                    VULKAN.create_frame_buffer(&create_info)
                };

                SwapchainImage { view, frame_buffer }
            });
        }
    }

    fn copy_data_to_gpu(frame: &mut Frame, vertices: &[Vertex], indices: &[u16]) -> vk::DeviceSize {
        let alignment = VULKAN.gpu_properties.limits.non_coherent_atom_size as usize;
        let vertex_buffer_size = ((std::mem::size_of_val(vertices) + alignment - 1) / alignment) * alignment;
        let min_capacity = (vertex_buffer_size + std::mem::size_of_val(indices)).max(DEFAULT_VERTEX_BUFFER_SIZE) as u64;

        if frame.buffer_size < min_capacity {
            VULKAN.destroy_buffer(frame.buffer);
            VULKAN.free(frame.memory);

            frame.buffer = VULKAN.create_buffer(&vk::BufferCreateInfo {
                s_type: vk::StructureType::BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::BufferCreateFlags::empty(),
                size: min_capacity,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
            });

            let memory_requirements = VULKAN.buffer_memory_requirements(frame.buffer);
            let memory_type_index = VULKAN
                .find_memory_type(
                    memory_requirements.memory_type_bits,
                    vk::MemoryPropertyFlags::HOST_VISIBLE,
                )
                .unwrap();

            let alloc_info = vk::MemoryAllocateInfo {
                s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                allocation_size: memory_requirements.size,
                memory_type_index,
            };

            frame.memory = VULKAN.allocate(&alloc_info);
            frame.buffer_size = memory_requirements.size;
            VULKAN.bind(frame.buffer, frame.memory, 0);
        }

        let data = VULKAN.map(frame.memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty());

        unsafe {
            let buffer = std::slice::from_raw_parts_mut(data as *mut _, vertices.len());
            buffer.copy_from_slice(vertices);

            let buffer = std::slice::from_raw_parts_mut(data.add(vertex_buffer_size as usize) as *mut _, indices.len());
            buffer.copy_from_slice(indices);
        }

        // PERFORMANCE(David Z): This call is unecessary if the memory is
        // host-coherent
        VULKAN.flush_mapped(&[vk::MappedMemoryRange {
            s_type: vk::StructureType::MAPPED_MEMORY_RANGE,
            p_next: std::ptr::null(),
            memory: frame.memory,
            offset: 0,
            size: vk::WHOLE_SIZE,
        }]);

        VULKAN.unmap(frame.memory);
        vertex_buffer_size as vk::DeviceSize
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        let fences = [self.frames[0].fence, self.frames[1].fence];
        let _ = VULKAN.wait_for_fences(&fences, u64::MAX);

        for frame in &self.frames {
            VULKAN.free_fence(frame.fence);
            VULKAN.free_semaphore(frame.acquire);
            VULKAN.free_semaphore(frame.present);
        }

        self.images.clear();

        VULKAN.free_command_buffers(
            self.command_pool,
            &[self.frames[0].command_buffer, self.frames[1].command_buffer],
        );
        VULKAN.destroy_command_pool(self.command_pool);

        VULKAN.destroy_pipeline(self.pipeline);
        VULKAN.destroy_render_pass(self.render_pass);

        VULKAN.destroy_swapchain(std::mem::take(&mut self.swapchain));
        VULKAN.destroy_surface(self.surface);
    }
}
