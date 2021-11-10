use ash::{vk, Device};

pub struct CommandRecorder<'a> {
    device: &'a Device,
    pub buffer: vk::CommandBuffer,
}

impl<'a> CommandRecorder<'a> {
    pub(crate) fn new(device: &'a ash::Device, buffer: vk::CommandBuffer) -> Self {
        Self { device, buffer }
    }

    pub fn begin(&self) {
        let begin_info = vk::CommandBufferBeginInfo::default();
        unsafe {
            self.device
                .begin_command_buffer(self.buffer, &begin_info)
                .expect("Out of memory");
        }
    }

    pub fn end(&self) {
        unsafe {
            self.device
                .end_command_buffer(self.buffer)
                .expect("Out of memory");
        }
    }

    pub fn begin_render_pass(
        &self,
        render_pass_info: &vk::RenderPassBeginInfo,
        subpass_contents: vk::SubpassContents,
    ) {
        unsafe {
            self.device
                .cmd_begin_render_pass(self.buffer, render_pass_info, subpass_contents);
        }
    }

    pub fn end_render_pass(&self) {
        unsafe {
            self.device.cmd_end_render_pass(self.buffer);
        }
    }

    pub fn bind_pipeline(&self, bind_point: vk::PipelineBindPoint, pipeline: vk::Pipeline) {
        unsafe {
            self.device
                .cmd_bind_pipeline(self.buffer, bind_point, pipeline);
        }
    }

    pub fn bind_vertex_buffers(&self, first: u32, vertex_buffers: &[vk::Buffer], offsets: &[u64]) {
        unsafe {
            self.device
                .cmd_bind_vertex_buffers(self.buffer, first, vertex_buffers, offsets);
        }
    }

    pub fn bind_index_buffer(&self, buffer: vk::Buffer, offset: u64, index_type: vk::IndexType) {
        unsafe {
            self.device
                .cmd_bind_index_buffer(self.buffer, buffer, offset, index_type);
        }
    }

    pub fn set_viewport(&self, viewports: &[vk::Viewport]) {
        unsafe {
            self.device.cmd_set_viewport(self.buffer, 0, viewports);
        }
    }

    pub fn set_scissor(&self, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device.cmd_set_scissor(self.buffer, 0, scissors);
        }
    }

    pub fn push_constants<T>(
        &self,
        layout: vk::PipelineLayout,
        stage: vk::ShaderStageFlags,
        offset: u32,
        constant: &T,
    ) {
        unsafe {
            let bytes = std::slice::from_raw_parts(
                constant as *const T as *const u8,
                std::mem::size_of::<T>(),
            );
            self.device
                .cmd_push_constants(self.buffer, layout, stage, offset, bytes);
        }
    }

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.cmd_draw_indexed(
                self.buffer,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }
}
