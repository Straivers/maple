use std::marker::PhantomData;

use ash::vk;

use crate::constants::{DEFAULT_GPU_BUFFER_SIZE, FRAMES_IN_FLIGHT};
use crate::effect::{Effect, EffectBase};
use sys::{dpi::PhysicalSize, window_handle::WindowHandle};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct Frame<VertexType: Copy> {
    pub image_view: vk::ImageView,
    pub image_format: vk::Format,
    pub frame_buffer: vk::Framebuffer,
    pub command_buffer: vk::CommandBuffer,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    memory_size: vk::DeviceSize,
    index_buffer_offset: vk::DeviceSize,
    _phantom_vt: PhantomData<VertexType>,
}

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct FrameSync {
    pub fence: vk::Fence,
    pub acquire_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore,
}

impl<VertexType: Copy> Frame<VertexType> {
    fn new(
        context: &vulkan_utils::Context,
        image: vk::Image,
        image_size: vk::Extent2D,
        image_format: vk::Format,
        command_pool: vk::CommandPool,
        render_pass: vk::RenderPass,
    ) -> Frame<VertexType> {
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

            context.create_image_view(&create_info)
        };

        let frame_buffer = {
            let attachment = [image_view];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&attachment)
                .width(image_size.width)
                .height(image_size.height)
                .layers(1);

            context.create_frame_buffer(&create_info)
        };

        let command_buffer = {
            let mut buffers = [vk::CommandBuffer::null()];
            context.allocate_command_buffers(command_pool, &mut buffers);
            buffers[0]
        };

        let mut frame = Self {
            image_view,
            image_format,
            frame_buffer,
            command_buffer,
            buffer: vk::Buffer::null(),
            memory: vk::DeviceMemory::null(),
            memory_size: 0,
            index_buffer_offset: 0,
            _phantom_vt: PhantomData,
        };

        frame.ensure_buffer_size(context, DEFAULT_GPU_BUFFER_SIZE);
        frame
    }

    pub fn vertex_buffer(&self) -> (vk::Buffer, vk::DeviceSize) {
        (self.buffer, 0)
    }

    pub fn index_buffer(&self) -> (vk::Buffer, vk::DeviceSize) {
        (self.buffer, self.index_buffer_offset)
    }

    pub fn copy_data_to_gpu(&mut self, context: &mut vulkan_utils::Context, vertices: &[VertexType], indices: &[u16]) {
        let alignment = context.gpu_properties.limits.non_coherent_atom_size;
        let vertex_buffer_size = round_size_to_multiple_of(std::mem::size_of_val(vertices) as u64, alignment);

        let min_capacity =
            vertices.len() * std::mem::size_of::<VertexType>() + indices.len() * std::mem::size_of::<u16>();

        self.ensure_buffer_size(context, min_capacity);

        let ptr = context.map(self.memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty());

        unsafe {
            let buffer = std::slice::from_raw_parts_mut(ptr as *mut _, vertices.len());
            buffer.copy_from_slice(vertices);

            let buffer = std::slice::from_raw_parts_mut(ptr.add(vertex_buffer_size as usize) as *mut _, indices.len());
            buffer.copy_from_slice(indices);
        }

        // PERFORMANCE: This call is unecessary if the memory is host-coherent
        context.flush_mapped(&[vk::MappedMemoryRange {
            s_type: vk::StructureType::MAPPED_MEMORY_RANGE,
            p_next: std::ptr::null(),
            memory: self.memory,
            offset: 0,
            size: vk::WHOLE_SIZE,
        }]);

        context.unmap(self.memory);

        self.index_buffer_offset = vertex_buffer_size;
    }

    fn destroy(self, context: &vulkan_utils::Context, command_pool: vk::CommandPool) {
        context.destroy_image_view(self.image_view);
        context.destroy_frame_buffer(self.frame_buffer);
        context.free_command_buffers(command_pool, &[self.command_buffer]);

        context.destroy_buffer(self.buffer);
        context.free(self.memory);
    }

    fn ensure_buffer_size(&mut self, context: &vulkan_utils::Context, size: usize) {
        if self.memory_size >= size as u64 {
            return;
        }

        context.destroy_buffer(self.buffer);
        context.free(self.memory);

        self.buffer = context.create_buffer(&vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size as u64,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        });

        let memory_requirements = context.buffer_memory_requirements(self.buffer);
        let memory_type_index = context
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

        self.memory = context.allocate(&alloc_info);
        self.memory_size = memory_requirements.size;
        context.bind(self.buffer, self.memory, 0);
    }
}

pub struct WindowContext<VertexType: Copy> {
    current_image: u32,
    current_frame: usize,
    surface: vk::SurfaceKHR,
    swapchain: vulkan_utils::SwapchainData,
    frames: Vec<Frame<VertexType>>,
    command_pool: vk::CommandPool,
    sync_objects: [FrameSync; FRAMES_IN_FLIGHT],
}

impl<VertexType: Copy> WindowContext<VertexType> {
    pub fn new(
        context: &mut vulkan_utils::Context,
        window_handle: WindowHandle,
        framebuffer_size: PhysicalSize,
        presentation_effect: &mut dyn EffectBase,
    ) -> Self {
        let surface = context.create_surface(window_handle);

        let swapchain = vulkan_utils::SwapchainData::new(context, surface, physical_size_to_extent(framebuffer_size));

        let command_pool = context.create_graphics_command_pool(true, true);

        let mut window_context = Self {
            current_image: 0,
            current_frame: 0,
            surface,
            swapchain,
            frames: Vec::new(),
            command_pool,
            sync_objects: [
                FrameSync {
                    fence: context.get_or_create_fence(true),
                    acquire_semaphore: context.get_or_create_semaphore(),
                    present_semaphore: context.get_or_create_semaphore(),
                },
                FrameSync {
                    fence: context.get_or_create_fence(true),
                    acquire_semaphore: context.get_or_create_semaphore(),
                    present_semaphore: context.get_or_create_semaphore(),
                },
            ],
        };

        let effect = presentation_effect.get_effect(context, window_context.swapchain.format);
        window_context.create_frames(context, effect.render_pass());
        window_context
    }

    pub fn destroy(mut self, context: &mut vulkan_utils::Context) {
        let fences = [self.sync_objects[0].fence, self.sync_objects[1].fence];
        let _ = context.wait_for_fences(&fences, u64::MAX);

        for frame in self.frames.drain(0..) {
            frame.destroy(context, self.command_pool);
        }

        self.swapchain.destroy(context);
        context.destroy_surface(self.surface);
        context.destroy_command_pool(self.command_pool);

        for sync in &self.sync_objects {
            context.free_fence(sync.fence);
            context.free_semaphore(sync.acquire_semaphore);
            context.free_semaphore(sync.present_semaphore);
        }
    }

    pub fn format(&self) -> vk::Format {
        self.swapchain.format
    }

    pub fn resize(
        &mut self,
        context: &vulkan_utils::Context,
        fb_size: PhysicalSize,
        presentation_effect: &mut dyn EffectBase,
    ) {
        let fences = [self.sync_objects[0].fence, self.sync_objects[1].fence];
        let _ = context.wait_for_fences(&fences, u64::MAX);

        let framebuffer_extent = physical_size_to_extent(fb_size);
        self.swapchain.resize(context, self.surface, framebuffer_extent);

        for frame in self.frames.drain(0..) {
            frame.destroy(context, self.command_pool);
        }

        let effect = presentation_effect.get_effect(context, self.swapchain.format);
        self.create_frames(context, effect.render_pass());
    }

    /// Returns None if resizing failed
    pub fn next_frame(
        &mut self,
        context: &vulkan_utils::Context,
        target_size: PhysicalSize,
        presentation_effect: &mut dyn EffectBase,
    ) -> Option<(&mut Frame<VertexType>, &FrameSync)> {
        let extent = physical_size_to_extent(target_size);

        let _ = context.wait_for_fences(&[self.sync_objects[self.current_frame].fence], u64::MAX);

        if extent != self.swapchain.image_size {
            self.resize(context, target_size, presentation_effect);
        }

        let acquire_semaphore = self.sync_objects[self.current_frame].acquire_semaphore;

        let image_index = {
            let index = context.get_swapchain_image(&self.swapchain, acquire_semaphore);
            if let Some(index) = index {
                index
            } else {
                self.resize(context, target_size, presentation_effect);
                context.get_swapchain_image(&self.swapchain, acquire_semaphore)?
            }
        };

        if self.swapchain.image_size == extent {
            self.current_image = image_index;
            let frame = &mut self.frames[image_index as usize];
            context.reset_command_buffer(frame.command_buffer, false);
            Some((frame, &self.sync_objects[self.current_frame]))
        } else {
            None
        }
    }

    pub fn present(&mut self, context: &vulkan_utils::Context) {
        context.present_swapchain_image(
            &self.swapchain,
            &[self.sync_objects[self.current_frame].present_semaphore],
            self.current_image,
        );

        self.current_frame = (self.current_frame + 1) % FRAMES_IN_FLIGHT;
    }

    fn create_frames(&mut self, context: &vulkan_utils::Context, render_pass: vk::RenderPass) {
        assert!(self.frames.is_empty());
        self.frames.reserve(self.swapchain.images.len());
        for image in &self.swapchain.images {
            self.frames.push(Frame::new(
                context,
                *image,
                self.swapchain.image_size,
                self.swapchain.format,
                self.command_pool,
                render_pass,
            ));
        }
    }
}

pub fn physical_size_to_extent(size: sys::dpi::PhysicalSize) -> vk::Extent2D {
    vk::Extent2D {
        width: u32::from(size.width),
        height: u32::from(size.height),
    }
}

fn round_size_to_multiple_of(from: vk::DeviceSize, multiple: vk::DeviceSize) -> vk::DeviceSize {
    ((from + multiple - 1) / multiple) * multiple
}
