use ash::vk;

use super::{
    shared::{
        create_pipeline, create_render_pass, record_command_buffer, to_extent, Request, Vertex,
        PIPELINE_LAYOUT, VULKAN,
    },
    vulkan::{SurfaceData, SwapchainData},
};
use crate::{shapes::Extent, sys::Handle};

pub const FRAMES_IN_FLIGHT: usize = 2;
pub const DEFAULT_VERTEX_BUFFER_SIZE: usize = 8192;
pub const MAX_SWAPCHAIN_DEPTH: usize = 8;

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

pub struct Frame {
    fence: vk::Fence,
    acquire: vk::Semaphore,
    present: vk::Semaphore,
    command_buffer: vk::CommandBuffer,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    buffer_size: vk::DeviceSize,
}

impl Frame {
    fn new(command_buffer: vk::CommandBuffer) -> Self {
        Self {
            fence: VULKAN.create_fence(true),
            acquire: VULKAN.create_semaphore(),
            present: VULKAN.create_semaphore(),
            command_buffer: command_buffer,
            buffer: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
            buffer_size: 0,
        }
    }
}

/// A [`RenderContext`] contains all render state needed for a window to
/// communicate with the renderer.
pub struct RendererWindow {
    surface: SurfaceData,
    swapchain: SwapchainData,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    images: Vec<SwapchainImage>,
    command_pool: vk::CommandPool,
    frames: [Frame; FRAMES_IN_FLIGHT],
    frame_id: u8,
}

impl RendererWindow {
    pub fn new(window: &Handle, window_size: Extent) -> Self {
        let surface = VULKAN.create_surface(window);
        let swapchain = VULKAN.create_or_resize_swapchain(&surface, to_extent(window_size), None);
        let render_pass = create_render_pass(swapchain.format);
        let pipeline = create_pipeline(*PIPELINE_LAYOUT, render_pass);
        let mut images = vec![];
        Self::init_images(&swapchain, render_pass, &mut images);
        let command_pool = VULKAN.create_graphics_command_pool(true, true);
        let mut command_buffers = [vk::CommandBuffer::null(), vk::CommandBuffer::null()];
        VULKAN.allocate_command_buffers(command_pool, &mut command_buffers);

        Self {
            surface,
            swapchain,
            render_pass,
            pipeline,
            images,
            command_pool,
            frames: [
                Frame::new(command_buffers[0]),
                Frame::new(command_buffers[1]),
            ],
            frame_id: 0,
        }
    }

    pub fn draw(
        &mut self,
        window_size: Extent,
        vertices: &[Vertex],
        indices: &[u16],
    ) -> Option<Request> {
        let window_extent = to_extent(window_size);
        if window_extent != self.swapchain.image_size {
            self.resize(window_extent);
        }

        let frame_id = self.frame_id as usize;
        let frame = &mut self.frames[frame_id];
        let _ = VULKAN.wait_for_fences(&[frame.fence], u64::MAX);

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

        let image_index = VULKAN.acquire_swapchain_image(&self.swapchain, frame.acquire)?;

        let cmd = VULKAN.record_command_buffer(frame.command_buffer);
        record_command_buffer(
            &cmd,
            viewport,
            self.pipeline,
            self.render_pass,
            *PIPELINE_LAYOUT,
            self.images[image_index as usize].frame_buffer,
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

    fn resize(&mut self, window_extent: vk::Extent2D) {
        // Wait for BOTH fences.
        let fences = [self.frames[0].fence, self.frames[1].fence];
        let _ = VULKAN.wait_for_fences(&fences, u64::MAX);

        let old_format = self.swapchain.format;
        self.swapchain = VULKAN.create_or_resize_swapchain(
            &self.surface,
            window_extent,
            Some(self.swapchain.handle),
        );

        if old_format != self.swapchain.format {
            VULKAN.destroy_pipeline(self.pipeline);
            VULKAN.destroy_render_pass(self.render_pass);

            self.render_pass = create_render_pass(self.swapchain.format);
            self.pipeline = create_pipeline(*PIPELINE_LAYOUT, self.render_pass);
        }

        self.images.clear();
        Self::init_images(&self.swapchain, self.render_pass, &mut self.images);
    }

    fn init_images(
        swapchain: &SwapchainData,
        render_pass: vk::RenderPass,
        buffer: &mut Vec<SwapchainImage>,
    ) {
        let images = VULKAN.get_swapchain_images::<MAX_SWAPCHAIN_DEPTH>(swapchain);
        buffer.reserve_exact(images.len());

        for handle in &images {
            buffer.push({
                let view = {
                    let create_info = vk::ImageViewCreateInfo::builder()
                        .image(*handle)
                        .format(swapchain.format)
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
                        .width(swapchain.image_size.width)
                        .height(swapchain.image_size.height)
                        .layers(1);

                    VULKAN.create_frame_buffer(&create_info)
                };

                SwapchainImage { view, frame_buffer }
            });
        }
    }

    fn copy_data_to_gpu(frame: &mut Frame, vertices: &[Vertex], indices: &[u16]) -> vk::DeviceSize {
        let alignment = VULKAN.non_coherent_atom_size() as usize;
        let vertex_buffer_size =
            ((std::mem::size_of_val(vertices) + alignment - 1) / alignment) * alignment;
        let min_capacity = (vertex_buffer_size + std::mem::size_of_val(indices))
            .max(DEFAULT_VERTEX_BUFFER_SIZE) as u64;

        if frame.buffer_size < min_capacity {
            VULKAN.destroy_buffer(frame.buffer);
            VULKAN.free(frame.memory);

            frame.buffer = VULKAN.create_buffer(&vk::BufferCreateInfo {
                size: min_capacity,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            });

            let memory_requirements = VULKAN.buffer_memory_requirements(frame.buffer);
            let memory_type_index = VULKAN
                .find_memory_type(
                    memory_requirements.memory_type_bits,
                    vk::MemoryPropertyFlags::HOST_VISIBLE,
                )
                .unwrap();

            let alloc_info = vk::MemoryAllocateInfo {
                allocation_size: memory_requirements.size,
                memory_type_index,
                ..Default::default()
            };

            frame.memory = VULKAN.allocate(&alloc_info);
            frame.buffer_size = memory_requirements.size;
            VULKAN.bind(frame.buffer, frame.memory, 0);
        }

        unsafe {
            let data =
                VULKAN.map_memory(frame.memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty());

            let vertex_buffer = std::slice::from_raw_parts_mut(data.cast(), vertices.len());
            vertex_buffer.copy_from_slice(vertices);

            let index_buffer = std::slice::from_raw_parts_mut(
                data.add(vertex_buffer_size as usize).cast(),
                indices.len(),
            );
            index_buffer.copy_from_slice(indices);

            // PERFORMANCE(David Z): This call is unecessary if the memory is
            // host-coherent
            VULKAN.flush_mapped_memory_ranges(&[vk::MappedMemoryRange {
                memory: frame.memory,
                offset: 0,
                size: vk::WHOLE_SIZE,
                ..Default::default()
            }]);

            VULKAN.unmap_memory(frame.memory);
        }

        vertex_buffer_size as vk::DeviceSize
    }
}

impl Drop for RendererWindow {
    fn drop(&mut self) {
        let fences = [self.frames[0].fence, self.frames[1].fence];
        let _ = VULKAN.wait_for_fences(&fences, u64::MAX);

        for frame in &self.frames {
            VULKAN.free_fence(frame.fence);
            VULKAN.free_semaphore(frame.acquire);
            VULKAN.free_semaphore(frame.present);
            VULKAN.destroy_buffer(frame.buffer);
            VULKAN.free(frame.memory);
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
        VULKAN.destroy_surface(std::mem::take(&mut self.surface));
    }
}
