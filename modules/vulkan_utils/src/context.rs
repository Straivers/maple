use std::{
    cmp::min,
    convert::TryInto,
    ffi::{c_void, CStr},
    iter::FromIterator,
    os::raw::c_char,
};

use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{Surface, Swapchain, Win32Surface},
    },
    // version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk,
    Device,
    EntryCustom,
    Instance,
};

use sys::library::Library;
use utils::array_vec::ArrayVec;

const MAX_PHYSICAL_DEVICES: usize = 16;
const MAX_QUEUE_FAMILIES: usize = 64;
const SYNC_POOL_SIZE: usize = 128;

const VALIDATION_LAYER_NAME: *const c_char = "VK_LAYER_KHRONOS_validation\0".as_ptr().cast();
const SURFACE_EXTENSION_NAME: *const c_char = "VK_KHR_surface\0".as_ptr().cast();
const DEBUG_UTILS_EXTENSION_NAME: *const c_char = "VK_EXT_debug_utils\0\0".as_ptr().cast();
const WIN32_SURFACE_EXTENSION_NAME: *const c_char = "VK_KHR_win32_surface\0".as_ptr().cast();
const SWAPCHAIN_EXTENSION_NAME: *const c_char = "VK_KHR_swapchain\0".as_ptr().cast();

pub struct VulkanDebug {
    api: DebugUtils,
    callback: vk::DebugUtilsMessengerEXT,
}

impl VulkanDebug {
    fn new(
        entry: &EntryCustom<Library>,
        instance: &Instance,
        create_info: &vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> Self {
        let api = DebugUtils::new(entry, instance);
        let callback = unsafe { api.create_debug_utils_messenger(create_info, None).unwrap() };
        Self { api, callback }
    }
}

pub struct Context {
    #[allow(dead_code)]
    library: EntryCustom<Library>,
    instance: Instance,
    pub(crate) gpu: Gpu,
    pub gpu_properties: vk::PhysicalDeviceProperties,
    pub gpu_memory_info: vk::PhysicalDeviceMemoryProperties,

    pub device: Device,

    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,

    pub surface_api: Surface,
    pub os_surface_api: Win32Surface,
    pub swapchain_api: Swapchain,

    pipeline_cache: vk::PipelineCache,
    fence_pool: ArrayVec<vk::Fence, SYNC_POOL_SIZE>,
    semaphore_pool: ArrayVec<vk::Semaphore, SYNC_POOL_SIZE>,

    debug: Option<VulkanDebug>,
}

impl Context {
    /// Initializes a new vulkan context.
    /// Note: The selected GPU is guaranteed to support surface creation.
    #[must_use]
    pub fn new(os_library: Library, use_validation: bool) -> Self {
        let library = EntryCustom::new_custom(os_library, |lib, name| {
            lib.get_symbol(name).unwrap_or(std::ptr::null_mut())
        })
        .expect("Loaded library does not contain Vuklan loader");

        let mut debug_callback_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(debug_callback));

        let instance = {
            let app_info = vk::ApplicationInfo::builder().api_version(vk::API_VERSION_1_2);
            let mut create_info = vk::InstanceCreateInfo::builder().application_info(&app_info);

            let mut layers = ArrayVec::<*const c_char, 1>::new();
            let mut extensions = ArrayVec::<_, 3>::from([SURFACE_EXTENSION_NAME, WIN32_SURFACE_EXTENSION_NAME]);

            let enables = [vk::ValidationFeatureEnableEXT::BEST_PRACTICES];
            let mut validation_features = vk::ValidationFeaturesEXT::builder().enabled_validation_features(&enables);

            if use_validation {
                layers.push(VALIDATION_LAYER_NAME);
                extensions.push(DEBUG_UTILS_EXTENSION_NAME);
                create_info = create_info.push_next(&mut debug_callback_create_info);
                create_info = create_info.push_next(&mut validation_features);
            }

            create_info = create_info
                .enabled_layer_names(layers.as_slice())
                .enabled_extension_names(extensions.as_slice());

            unsafe { library.create_instance(&create_info, None) }.expect("Unexpected error")
        };

        let debug = if use_validation {
            Some(VulkanDebug::new(&library, &instance, &debug_callback_create_info))
        } else {
            None
        };

        let surface_api = Surface::new(&library, &instance);
        let os_surface_api = Win32Surface::new(&library, &instance);

        let gpu = select_physical_device(&instance, &os_surface_api).expect("No supported GPU found");

        let gpu_properties = unsafe { instance.get_physical_device_properties(gpu.handle) };

        let gpu_memory_info = unsafe { instance.get_physical_device_memory_properties(gpu.handle) };

        let device = {
            let priorities = [1.0];
            let mut queue_create_infos = ArrayVec::<vk::DeviceQueueCreateInfo, 2>::new();
            queue_create_infos.push(
                *vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(gpu.graphics_queue_index)
                    .queue_priorities(&priorities),
            );

            if gpu.present_queue_index != gpu.graphics_queue_index {
                queue_create_infos.push(
                    *vk::DeviceQueueCreateInfo::builder()
                        .queue_family_index(gpu.present_queue_index)
                        .queue_priorities(&priorities),
                );
            }

            let features: vk::PhysicalDeviceFeatures = unsafe { std::mem::zeroed() };
            let extensions = ArrayVec::<_, 1>::from_iter([SWAPCHAIN_EXTENSION_NAME]);

            let create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(queue_create_infos.as_slice())
                .enabled_extension_names(extensions.as_slice())
                .enabled_features(&features);

            unsafe { instance.create_device(gpu.handle, &create_info, None) }.expect("Unexpected error")
        };

        let swapchain_api = Swapchain::new(&instance, &device);

        let present_queue = unsafe { device.get_device_queue(gpu.present_queue_index, 0) };
        let graphics_queue = unsafe { device.get_device_queue(gpu.graphics_queue_index, 0) };

        let pipeline_cache = {
            let create_info = vk::PipelineCacheCreateInfo::builder();
            // Only fails on out of memory (Vulkan 1.2; Aug 7, 2021)
            unsafe { device.create_pipeline_cache(&create_info, None) }.expect("Out of memory")
        };

        Self {
            library,
            instance,
            gpu,
            gpu_properties,
            gpu_memory_info,
            device,
            graphics_queue,
            present_queue,
            surface_api,
            os_surface_api,
            swapchain_api,
            pipeline_cache,
            fence_pool: ArrayVec::new(),
            semaphore_pool: ArrayVec::new(),
            debug,
        }
    }

    /// Fetches a fence from the context's pool, or creates a new one. If the
    /// fence needs to be signalled, a new one will be created.
    ///
    /// # Panics
    /// Panics on out of memory conditions
    #[must_use]
    pub fn get_or_create_fence(&mut self, signalled: bool) -> vk::Fence {
        if !self.fence_pool.is_empty() && !signalled {
            self.fence_pool.pop().unwrap()
        } else {
            let ci = vk::FenceCreateInfo {
                flags: if signalled {
                    vk::FenceCreateFlags::SIGNALED
                } else {
                    vk::FenceCreateFlags::empty()
                },
                ..Default::default()
            };

            unsafe { self.device.create_fence(&ci, None).expect("Out of memory") }
        }
    }

    /// Returns a fence to the context's pool, or destroys it if the fence pool
    /// is at capacity.
    pub fn free_fence(&mut self, fence: vk::Fence) {
        unsafe { self.device.reset_fences(&[fence]) }.expect("Vulkan out of memory");

        if self.fence_pool.is_full() {
            unsafe {
                self.device.destroy_fence(fence, None);
            }
        } else {
            self.fence_pool.push(fence);
        }
    }

    /// `true` of success, `false` for time out
    #[must_use]
    pub fn wait_for_fences(&self, fences: &[vk::Fence], timeout: u64) -> bool {
        let r = unsafe {
            self.device.fp_v1_0().wait_for_fences(
                self.device.handle(),
                fences.len() as u32,
                fences.as_ptr(),
                vk::TRUE,
                timeout,
            )
        };

        match r {
            vk::Result::SUCCESS => true,
            vk::Result::TIMEOUT => false,
            any => panic!("Unexpected error: {:?}", any),
        }
    }

    pub fn reset_fences(&self, fences: &[vk::Fence]) {
        unsafe {
            self.device.reset_fences(fences).expect("Out of memory");
        }
    }

    /// Fetches a semaphore from the context's pool, or creates a new one.
    ///
    /// # Panics
    /// Panics on out of memory conditions
    #[must_use]
    pub fn get_or_create_semaphore(&mut self) -> vk::Semaphore {
        self.semaphore_pool.pop().unwrap_or_else(|| {
            let ci = vk::SemaphoreCreateInfo::builder();
            unsafe { self.device.create_semaphore(&ci, None) }.expect("Out of memory")
        })
    }

    /// Returns a semaphore to the context's pool, or destroys it if the
    /// semaphore pool is at capacity.
    pub fn free_semaphore(&mut self, semaphore: vk::Semaphore) {
        if self.semaphore_pool.is_full() {
            unsafe {
                self.device.destroy_semaphore(semaphore, None);
            }
        } else {
            self.semaphore_pool.push(semaphore);
        }
    }

    /// Creates a new shader from SPIR-V source. Note that the source must be
    /// 4-byte aligned to be accepted as valid.
    /// # Panics
    /// Panics on out of memory conditions
    #[must_use]
    pub fn create_shader(&self, source: &[u8]) -> vk::ShaderModule {
        if source.len() % 4 == 0 && ((source.as_ptr() as usize) % 4) == 0 {
            let words = unsafe { std::slice::from_raw_parts(source.as_ptr().cast(), source.len() / 4) };
            let ci = vk::ShaderModuleCreateInfo::builder().code(words);

            // Only fails on out of memory, or unused extension errors (Vulkan
            // 1.2; Aug 7, 2021)
            unsafe { self.device.create_shader_module(&ci, None) }.expect("Out of memory")
        } else {
            panic!("Shader source must be aligned to 4-byte words")
        }
    }

    pub fn destroy_shader(&self, shader: vk::ShaderModule) {
        unsafe {
            self.device.destroy_shader_module(shader, None);
        }
    }

    /// # Panics
    /// Panics on out of memory conditions
    #[must_use]
    pub fn create_pipeline_layout(&self, create_info: &vk::PipelineLayoutCreateInfo) -> vk::PipelineLayout {
        // Only fails on out of memory (Vulkan 1.2; Aug 7, 2021)
        unsafe { self.device.create_pipeline_layout(create_info, None) }.expect("Out of memory")
    }

    pub fn destroy_pipeline_layout(&self, pipeline_layout: vk::PipelineLayout) {
        unsafe {
            self.device.destroy_pipeline_layout(pipeline_layout, None);
        }
    }

    /// # Panics
    /// Panics on out of memory conditions
    #[must_use]
    pub fn create_graphics_pipeline(&self, create_info: &vk::GraphicsPipelineCreateInfo) -> vk::Pipeline {
        let mut pipeline = vk::Pipeline::default();

        // Only fails on out of memory (Vulkan 1.2; Aug 7, 2021)
        unsafe {
            self.device
                .fp_v1_0()
                .create_graphics_pipelines(
                    self.device.handle(),
                    vk::PipelineCache::null(),
                    1,
                    create_info,
                    std::ptr::null(),
                    &mut pipeline,
                )
                .result()
                .expect("Out of memory");
        }

        pipeline
    }

    pub fn destroy_pipeline(&self, pipeline: vk::Pipeline) {
        unsafe {
            self.device.destroy_pipeline(pipeline, None);
        }
    }

    /// # Panics
    /// Panics on out of memory conditions
    #[must_use]
    pub fn create_graphics_command_pool(&self, transient: bool, reset_individual: bool) -> vk::CommandPool {
        let mut create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.gpu.graphics_queue_index)
            .build();

        if transient {
            create_info.flags |= vk::CommandPoolCreateFlags::TRANSIENT;
        }

        if reset_individual {
            create_info.flags |= vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER;
        }

        // Only fails on out of memory (Vulkan 1.2; Aug 7, 2021)
        unsafe { self.device.create_command_pool(&create_info, None) }.expect("Out of memory")
    }

    pub fn reset_command_pool(&self, pool: vk::CommandPool, release_memory: bool) {
        unsafe {
            self.device
                .reset_command_pool(
                    pool,
                    if release_memory {
                        vk::CommandPoolResetFlags::RELEASE_RESOURCES
                    } else {
                        vk::CommandPoolResetFlags::empty()
                    },
                )
                .expect("Out of memory");
        }
    }

    pub fn destroy_command_pool(&self, pool: vk::CommandPool) {
        unsafe {
            self.device.destroy_command_pool(pool, None);
        }
    }

    pub fn allocate_command_buffers(&self, pool: vk::CommandPool, buffers: &mut [vk::CommandBuffer]) {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(buffers.len() as u32)
            .build();

        unsafe {
            self.device
                .fp_v1_0()
                .allocate_command_buffers(self.device.handle(), &alloc_info, buffers.as_mut_ptr())
                .result()
                .expect("Out of memory");
        }
    }

    pub fn free_command_buffers(&self, command_pool: vk::CommandPool, command_buffers: &[vk::CommandBuffer]) {
        unsafe {
            self.device.free_command_buffers(command_pool, command_buffers);
        }
    }

    pub fn reset_command_buffer(&self, buffer: vk::CommandBuffer, release_memory: bool) {
        let mut flags = Default::default();

        if release_memory {
            flags |= vk::CommandBufferResetFlags::RELEASE_RESOURCES;
        }

        unsafe {
            self.device
                .reset_command_buffer(buffer, flags)
                .expect("Out of device memory");
        }
    }

    pub fn submit_to_graphics_queue(&self, submits: &[vk::SubmitInfo], fence: vk::Fence) {
        unsafe {
            self.device
                .queue_submit(self.graphics_queue, submits, fence)
                .expect("Unexpected error");
        }
    }

    #[must_use]
    pub fn create_image_view(&self, create_info: &vk::ImageViewCreateInfo) -> vk::ImageView {
        unsafe { self.device.create_image_view(create_info, None) }.expect("Out of memory")
    }

    pub fn destroy_image_view(&self, view: vk::ImageView) {
        unsafe {
            self.device.destroy_image_view(view, None);
        }
    }

    #[must_use]
    pub fn create_frame_buffer(&self, create_info: &vk::FramebufferCreateInfo) -> vk::Framebuffer {
        unsafe { self.device.create_framebuffer(create_info, None) }.expect("Out of memory")
    }

    pub fn destroy_frame_buffer(&self, framebuffer: vk::Framebuffer) {
        unsafe {
            self.device.destroy_framebuffer(framebuffer, None);
        }
    }

    #[must_use]
    pub fn create_render_pass(&self, create_info: &vk::RenderPassCreateInfo) -> vk::RenderPass {
        unsafe { self.device.create_render_pass(create_info, None) }.expect("Out of memory")
    }

    pub fn destroy_render_pass(&self, renderpass: vk::RenderPass) {
        unsafe {
            self.device.destroy_render_pass(renderpass, None);
        }
    }

    pub fn create_buffer(&self, create_info: &vk::BufferCreateInfo) -> vk::Buffer {
        unsafe { self.device.create_buffer(create_info, None).expect("Out of memory") }
    }

    pub fn destroy_buffer(&self, buffer: vk::Buffer) {
        unsafe {
            self.device.destroy_buffer(buffer, None);
        }
    }

    pub fn buffer_memory_requirements(&self, buffer: vk::Buffer) -> vk::MemoryRequirements {
        unsafe { self.device.get_buffer_memory_requirements(buffer) }
    }

    pub fn find_memory_type(&self, type_filter: u32, needed_properties: vk::MemoryPropertyFlags) -> Option<u32> {
        for i in 0..self.gpu_memory_info.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && self.gpu_memory_info.memory_types[i as usize]
                    .property_flags
                    .contains(needed_properties)
            {
                return Some(i);
            }
        }

        None
    }

    pub fn allocate(&self, alloc_info: &vk::MemoryAllocateInfo) -> vk::DeviceMemory {
        unsafe { self.device.allocate_memory(alloc_info, None).expect("Out of memory") }
    }

    pub fn free(&self, memory: vk::DeviceMemory) {
        unsafe {
            self.device.free_memory(memory, None);
        }
    }

    pub fn bind(&self, buffer: vk::Buffer, memory: vk::DeviceMemory, offset: u64) {
        unsafe {
            self.device
                .bind_buffer_memory(buffer, memory, offset)
                .expect("Out of memory");
        }
    }

    pub fn map(&self, memory: vk::DeviceMemory, offset: vk::DeviceSize, size: vk::DeviceSize, flags: vk::MemoryMapFlags) -> *mut c_void {
        unsafe {
            self.device.map_memory(memory, offset, size,flags).expect("Memory map failed")
        }
    }

    pub fn flush_mapped(&self, ranges: &[vk::MappedMemoryRange]) {
        unsafe {
            self.device.flush_mapped_memory_ranges(ranges).expect("Out of memory");
        }
    }

    pub fn unmap(&self, memory: vk::DeviceMemory) {
        unsafe {
            self.device.unmap_memory(memory);
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // We're shutting down, so ignore errors
            let _ = self.device.device_wait_idle();

            std::mem::take(&mut self.fence_pool).empty(|fence| self.device.destroy_fence(fence, None));
            std::mem::take(&mut self.semaphore_pool).empty(|sem| self.device.destroy_semaphore(sem, None));

            if let Some(debug) = self.debug.as_ref() {
                debug.api.destroy_debug_utils_messenger(debug.callback, None);
            }

            self.device.destroy_pipeline_cache(self.pipeline_cache, None);

            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    if severity < vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        return vk::FALSE;
    }

    println!("Vulkan: {:?}", CStr::from_ptr((*callback_data).p_message));

    vk::FALSE
}

pub(crate) struct Gpu {
    pub handle: vk::PhysicalDevice,
    pub graphics_queue_index: u32,
    pub present_queue_index: u32,
}

fn select_physical_device(instance: &Instance, surface_api: &Win32Surface) -> Option<Gpu> {
    let physical_devices = load_vk_objects::<_, _, MAX_PHYSICAL_DEVICES>(|count, ptr| unsafe {
        instance
            .fp_v1_0()
            .enumerate_physical_devices(instance.handle(), count, ptr)
    });

    let physical_devices = if let Ok(devices) = physical_devices {
        if devices.is_empty() {
            return None;
        }
        devices
    } else {
        return None;
    };

    for physical_device in &physical_devices {
        let queue_families = load_vk_objects::<_, _, MAX_QUEUE_FAMILIES>(|count, ptr| {
            unsafe {
                instance
                    .fp_v1_0()
                    .get_physical_device_queue_family_properties(*physical_device, count, ptr);
            }
            vk::Result::SUCCESS
        })
        // Always passes because we always return VkSuccess
        .unwrap();

        let mut graphics = None;
        let mut present = None;
        for (queue_family_index, properties) in queue_families.iter().enumerate() {
            if properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics = Some(queue_family_index);
            }

            if unsafe {
                surface_api.get_physical_device_win32_presentation_support(
                    *physical_device,
                    queue_family_index.try_into().unwrap(),
                )
            } {
                present = Some(queue_family_index);
            }

            if let Some((graphics_i, present_i)) = graphics.zip(present) {
                return Some(Gpu {
                    handle: *physical_device,
                    graphics_queue_index: graphics_i.try_into().unwrap(),
                    present_queue_index: present_i.try_into().unwrap(),
                });
            }
        }
    }

    None
}

pub(crate) fn load_vk_objects<T, F, const COUNT: usize>(mut func: F) -> Result<ArrayVec<T, COUNT>, vk::Result>
where
    F: FnMut(*mut u32, *mut T) -> vk::Result,
{
    let mut count = 0;
    func(&mut count, std::ptr::null_mut()).result()?;
    let mut buffer = ArrayVec::new();
    count = min(count, buffer.capacity().try_into().unwrap());
    func(&mut count, buffer.as_mut_ptr()).result()?;

    unsafe {
        buffer.set_len(count as usize);
    }

    Ok(buffer)
}
