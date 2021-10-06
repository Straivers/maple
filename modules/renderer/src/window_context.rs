use std::marker::PhantomData;

use ash::vk;
use vulkan_utils::Vulkan;

use crate::constants::{DEFAULT_GPU_BUFFER_SIZE, FRAMES_IN_FLIGHT};
use sys::window_handle::WindowHandle;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub image_view: vk::ImageView,
    pub image_format: vk::Format,
    pub frame_buffer: vk::Framebuffer,
}

impl Frame {
    fn new(
        vulkan: &Vulkan,
        image: vk::Image,
        image_size: vk::Extent2D,
        image_format: vk::Format,
        render_pass: vk::RenderPass,
    ) -> Self {
        let image_view = {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(image)
                .format(image_format)
                .view_type(vk::ImageViewType::TYPE_2D)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            vulkan.create_image_view(&create_info)
        };

        let frame_buffer = {
            let attachment = [image_view];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&attachment)
                .width(image_size.width)
                .height(image_size.height)
                .layers(1);

            vulkan.create_frame_buffer(&create_info)
        };

        Self {
            image_view,
            image_format,
            frame_buffer,
        }
    }

    fn destroy(self, vulkan: &Vulkan) {
        vulkan.destroy_image_view(self.image_view);
        vulkan.destroy_frame_buffer(self.frame_buffer);
    }
}

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct FrameObjects<VertexType: Copy> {
    pub fence: vk::Fence,
    pub acquire_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore,
    pub command_buffer: vk::CommandBuffer,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    memory_size: vk::DeviceSize,
    index_buffer_offset: vk::DeviceSize,
    phantom: PhantomData<VertexType>,
}

impl<VertexType: Copy> FrameObjects<VertexType> {
    fn new(vulkan: &mut Vulkan, command_buffer: vk::CommandBuffer) -> Self {
        let mut objects = Self {
            fence: vulkan.get_or_create_fence(true),
            acquire_semaphore: vulkan.get_or_create_semaphore(),
            present_semaphore: vulkan.get_or_create_semaphore(),
            command_buffer,
            buffer: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
            memory_size: 0,
            index_buffer_offset: 0,
            phantom: PhantomData,
        };

        objects.ensure_buffer_size(vulkan, DEFAULT_GPU_BUFFER_SIZE);
        objects
    }

    fn destroy(self, vulkan: &mut Vulkan) -> vk::CommandBuffer {
        vulkan.free_fence(self.fence);
        vulkan.free_semaphore(self.acquire_semaphore);
        vulkan.free_semaphore(self.present_semaphore);

        vulkan.destroy_buffer(self.buffer);
        vulkan.free(self.memory);
        self.command_buffer
    }

    pub fn vertex_buffer(&self) -> (vk::Buffer, vk::DeviceSize) {
        (self.buffer, 0)
    }

    pub fn index_buffer(&self) -> (vk::Buffer, vk::DeviceSize) {
        (self.buffer, self.index_buffer_offset)
    }

    pub fn copy_data_to_gpu(&mut self, vulkan: &Vulkan, vertices: &[VertexType], indices: &[u16]) {
        let alignment = vulkan.gpu_properties.limits.non_coherent_atom_size as usize;
        let vertex_buffer_size = ((std::mem::size_of_val(vertices) + alignment - 1) / alignment) * alignment;
        let min_capacity = vertex_buffer_size + std::mem::size_of_val(indices);

        self.ensure_buffer_size(vulkan, min_capacity);

        let ptr = vulkan.map(self.memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty());

        unsafe {
            let buffer = std::slice::from_raw_parts_mut(ptr as *mut _, vertices.len());
            buffer.copy_from_slice(vertices);

            let buffer = std::slice::from_raw_parts_mut(ptr.add(vertex_buffer_size as usize) as *mut _, indices.len());
            buffer.copy_from_slice(indices);
        }

        // PERFORMANCE: This call is unecessary if the memory is host-coherent
        vulkan.flush_mapped(&[vk::MappedMemoryRange {
            s_type: vk::StructureType::MAPPED_MEMORY_RANGE,
            p_next: std::ptr::null(),
            memory: self.memory,
            offset: 0,
            size: vk::WHOLE_SIZE,
        }]);

        vulkan.unmap(self.memory);

        self.index_buffer_offset = vertex_buffer_size as u64;
    }

    fn ensure_buffer_size(&mut self, vulkan: &Vulkan, size: usize) {
        if self.memory_size >= size as u64 {
            return;
        }

        vulkan.destroy_buffer(self.buffer);
        vulkan.free(self.memory);

        self.buffer = vulkan.create_buffer(&vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size as u64,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        });

        let memory_requirements = vulkan.buffer_memory_requirements(self.buffer);
        let memory_type_index = vulkan
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

        self.memory = vulkan.allocate(&alloc_info);
        self.memory_size = memory_requirements.size;
        vulkan.bind(self.buffer, self.memory, 0);
    }
}

pub struct WindowContext<VertexType: Copy> {
    current_image: u32,
    current_frame: usize,
    surface: vk::SurfaceKHR,
    swapchain: vulkan_utils::SwapchainData,
    frames: Vec<Frame>,
    command_pool: vk::CommandPool,
    sync_objects: [FrameObjects<VertexType>; FRAMES_IN_FLIGHT],
}

impl<VertexType: Copy> WindowContext<VertexType> {
    pub fn new(vulkan: &mut Vulkan, window_handle: WindowHandle, window_extent: vk::Extent2D) -> Self {
        let surface = vulkan.create_surface(window_handle);
        let swapchain = vulkan.create_swapchain(surface, window_extent);

        let command_pool = vulkan.create_graphics_command_pool(true, true);
        let mut command_buffers = [vk::CommandBuffer::null(), vk::CommandBuffer::null()];
        vulkan.allocate_command_buffers(command_pool, &mut command_buffers);

        Self {
            current_image: 0,
            current_frame: 0,
            surface,
            swapchain,
            frames: Vec::new(),
            command_pool,
            sync_objects: [
                FrameObjects::new(vulkan, command_buffers[0]),
                FrameObjects::new(vulkan, command_buffers[1]),
            ],
        }
    }

    pub fn destroy(mut self, vulkan: &mut Vulkan) {
        let fences = [self.sync_objects[0].fence, self.sync_objects[1].fence];
        let _ = vulkan.wait_for_fences(&fences, u64::MAX);

        for frame in self.frames.drain(0..) {
            frame.destroy(vulkan);
        }

        vulkan.destroy_swapchain(self.swapchain);
        vulkan.destroy_surface(self.surface);

        let command_buffers = [
            self.sync_objects[0].destroy(vulkan),
            self.sync_objects[1].destroy(vulkan),
        ];

        vulkan.free_command_buffers(self.command_pool, &command_buffers);
        vulkan.destroy_command_pool(self.command_pool);
    }

    pub fn format(&self) -> vk::Format {
        self.swapchain.format
    }

    /// Recreates the swapchain's framebuffers after a call to `next_frame()`
    /// returns [None] to indicate a resize operation.
    pub fn update_render_pass(&mut self, vulkan: &Vulkan, render_pass: vk::RenderPass) {
        assert!(self.frames.is_empty());
        self.frames.reserve(self.swapchain.images.len());
        for image in &self.swapchain.images {
            self.frames.push(Frame::new(
                vulkan,
                *image,
                self.swapchain.image_size,
                self.swapchain.format,
                render_pass,
            ));
        }
    }

    /// Retrieves the next frame for this [WindowContext].
    ///
    /// If the window was resized since the last call to `next_frame()`, this
    /// function will return [None]. If this happens, the surface's format may
    /// have changed. Call `update_render_pass()` with a compatible render pass
    /// before calling `next_frame()` again. It is a bug for it to fail a
    /// second time.
    pub fn next_frame(
        &mut self,
        vulkan: &Vulkan,
        window_extent: vk::Extent2D,
    ) -> Option<(&Frame, &mut FrameObjects<VertexType>)> {
        assert!(!self.frames.is_empty(), "WindowContext::next_frame() was called with no frames, call update_render_pass() to create swapchain framebuffers!");

        let _ = vulkan.wait_for_fences(&[self.sync_objects[self.current_frame].fence], u64::MAX);

        if window_extent != self.swapchain.image_size {
            self.resize(vulkan, window_extent);
            return None;
        }

        let acquire_semaphore = self.sync_objects[self.current_frame].acquire_semaphore;

        let image_index = if let Some(index) = vulkan.get_swapchain_image(&self.swapchain, acquire_semaphore) {
            index
        } else {
            self.resize(vulkan, window_extent);
            return None;
        };

        self.current_image = image_index;
        let frame = &self.frames[image_index as usize];
        let objects = &mut self.sync_objects[self.current_frame];
        vulkan.reset_command_buffer(objects.command_buffer, false);
        Some((frame, objects))
    }

    pub fn present(&mut self, vulkan: &Vulkan) {
        vulkan.present_swapchain_image(
            &self.swapchain,
            &[self.sync_objects[self.current_frame].present_semaphore],
            self.current_image,
        );

        self.current_frame = (self.current_frame + 1) % FRAMES_IN_FLIGHT;
    }

    fn resize(&mut self, vulkan: &Vulkan, window_extent: vk::Extent2D) {
        let fences = [self.sync_objects[0].fence, self.sync_objects[1].fence];
        let _ = vulkan.wait_for_fences(&fences, u64::MAX);

        // self.swapchain.resize(vulkan, self.surface, window_extent);
        let old = Some((self.swapchain.handle, std::mem::take(&mut self.swapchain.images)));
        self.swapchain = vulkan.resize_swapchain(self.surface, window_extent, old);

        for frame in self.frames.drain(0..) {
            frame.destroy(vulkan);
        }
    }
}
