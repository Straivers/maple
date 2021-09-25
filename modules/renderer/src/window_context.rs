use std::marker::PhantomData;

use ash::vk;

use crate::constants::{DEFAULT_VERTEX_BUFFER_SIZE, FRAMES_IN_FLIGHT};
use crate::effect::{Effect, EffectBase};
use sys::{dpi::PhysicalSize, window_handle::WindowHandle};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct Frame<VertexType> {
    pub image_view: vk::ImageView,
    pub image_format: vk::Format,
    pub frame_buffer: vk::Framebuffer,
    pub command_buffer: vk::CommandBuffer,
    vertex_buffer: vk::Buffer,
    vertex_memory: vk::DeviceMemory,
    vertex_buffer_size: vk::DeviceSize,
    _phantom_vt: PhantomData<VertexType>,
}

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct FrameSync {
    pub fence: vk::Fence,
    pub acquire_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore,
}

impl<VertexType> Frame<VertexType> {
    fn new(
        context: &mut vulkan_utils::Context,
        image: vk::Image,
        image_size: vk::Extent2D,
        image_format: vk::Format,
        command_pool: vk::CommandPool,
        effect: &dyn Effect,
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
                .render_pass(effect.render_pass())
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

        let (vertex_buffer, vertex_memory, vertex_buffer_size) = Self::create_buffer(
            context,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            std::mem::size_of::<VertexType>() * DEFAULT_VERTEX_BUFFER_SIZE,
        );

        Self {
            image_view,
            image_format,
            frame_buffer,
            command_buffer,
            vertex_buffer,
            vertex_memory,
            vertex_buffer_size,
            _phantom_vt: PhantomData,
        }
    }

    pub fn vertex_buffer(&self) -> vk::Buffer {
        self.vertex_buffer
    }

    pub fn reserve_vertex_buffer_capacity(&mut self, context: &mut vulkan_utils::Context, num_vertices: usize) {
        let min_capacity = num_vertices * std::mem::size_of::<VertexType>();

        if self.vertex_buffer_size < min_capacity as vk::DeviceSize {
            context.destroy_buffer(self.vertex_buffer);
            context.free(self.vertex_memory);

            let (buffer, memory, size) =
                Self::create_buffer(context, vk::BufferUsageFlags::VERTEX_BUFFER, min_capacity);
            self.vertex_buffer = buffer;
            self.vertex_memory = memory;
            self.vertex_buffer_size = size;
        }
    }

    pub fn map_vertices<'a, 'b: 'a>(&'a mut self, context: &'b mut vulkan_utils::Context) -> &'a mut [VertexType] {
        context.map_typed::<VertexType>(
            self.vertex_memory,
            0,
            self.vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )
    }

    pub fn unmap_vertices(&self, context: &vulkan_utils::Context) {
        context.unmap(self.vertex_memory);
    }

    fn destroy(self, context: &mut vulkan_utils::Context, command_pool: vk::CommandPool) {
        context.destroy_image_view(self.image_view);
        context.destroy_frame_buffer(self.frame_buffer);
        context.free_command_buffers(command_pool, &[self.command_buffer]);

        context.destroy_buffer(self.vertex_buffer);
        context.free(self.vertex_memory);
    }

    fn create_buffer(
        context: &mut vulkan_utils::Context,
        kind: vk::BufferUsageFlags,
        size: usize,
    ) -> (vk::Buffer, vk::DeviceMemory, vk::DeviceSize) {
        let create_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size as u64,
            usage: kind,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };

        let buffer = context.create_buffer(&create_info);

        let memory_requirements = context.buffer_memory_requirements(buffer);
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

        let buffer_memory = context.allocate(&alloc_info);
        context.bind(buffer, buffer_memory, 0);

        (buffer, buffer_memory, memory_requirements.size)
    }
}

pub struct WindowContext<VertexType> {
    current_image: u32,
    current_frame: usize,
    surface: vk::SurfaceKHR,
    swapchain: vulkan_utils::SwapchainData,
    frames: Vec<Frame<VertexType>>,
    command_pool: vk::CommandPool,
    sync_objects: [FrameSync; FRAMES_IN_FLIGHT],
}

impl<VertexType> WindowContext<VertexType> {
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
        window_context.create_frames(context, effect);
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
        context: &mut vulkan_utils::Context,
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
        self.create_frames(context, effect);
    }

    /// Returns None if resizing failed
    pub fn next_frame(
        &mut self,
        context: &mut vulkan_utils::Context,
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

    pub fn present(&mut self, context: &mut vulkan_utils::Context) {
        context.present_swapchain_image(
            &self.swapchain,
            &[self.sync_objects[self.current_frame].present_semaphore],
            self.current_image,
        );

        self.current_frame = (self.current_frame + 1) % FRAMES_IN_FLIGHT;
    }

    fn create_frames(&mut self, context: &mut vulkan_utils::Context, effect: &dyn Effect) {
        assert!(self.frames.is_empty());
        self.frames.reserve(self.swapchain.images.len());
        for image in &self.swapchain.images {
            self.frames.push(Frame::new(
                context,
                *image,
                self.swapchain.image_size,
                self.swapchain.format,
                self.command_pool,
                effect,
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
