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
    InstanceError::{LoadError, VkError},
};

use sys::library::Library;

use utils::array_vec::ArrayVec;

use super::error::{Error as VulkanError, Result};

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

pub struct Context {
    #[allow(dead_code)]
    library: EntryCustom<Library>,
    instance: Instance,
    pub(crate) gpu: Gpu,
    pub device: Device,

    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,

    pub surface_api: Surface,
    pub os_surface_api: Win32Surface,
    pub swapchain_api: Swapchain,

    pipeline_cache: vk::PipelineCache,
    fence_pool: ArrayVec<vk::Fence, SYNC_POOL_SIZE>,
    semaphore_pool: ArrayVec<vk::Semaphore, SYNC_POOL_SIZE>,
    pub graphics_command_pool: vk::CommandPool,

    debug: Option<VulkanDebug>,
}

impl Context {
    /// Initializes a new vulkan context.
    /// Note: The selected GPU is guaranteed to support surface creation.
    ///
    /// # Errors
    /// This function may fail for the following reasons:
    ///
    /// - No GPU that support both rendering and presentation was found
    /// - The Vulkan loader library could not be found on the system path
    /// - `VK_ERROR_OUT_OF_HOST_MEMORY`
    /// - `VK_ERROR_OUT_OF_DEVICE_MEMORY`
    /// - `VK_ERROR_INITIALIZATION_FAILED`
    /// - `VK_ERROR_EXTENSION_NOT_PRESENT`
    /// - `VK_ERROR_LAYER_NOT_PRESENT`
    /// - `VK_ERROR_FEATURE_NOT_PRESENT`
    /// - `VK_ERROR_TOO_MANY_OBJECTS`
    /// - `VK_ERROR_DEVICE_LOST`
    pub fn new(os_library: Library, use_validation: bool) -> Result<Self> {
        let library = {
            let entry = EntryCustom::new_custom(os_library, |lib, name| {
                lib.get_symbol(name).unwrap_or(std::ptr::null_mut())
            });

            if let Ok(e) = entry {
                e
            } else {
                return Err(VulkanError::LibraryNotVulkan);
            }
        };

        let mut debug_callback_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(debug_callback));

        let instance = {
            let app_info = vk::ApplicationInfo::builder().api_version(vk::API_VERSION_1_2);

            let mut layers = ArrayVec::<*const c_char, 1>::new();
            let mut extensions = ArrayVec::<_, 3>::from([SURFACE_EXTENSION_NAME, WIN32_SURFACE_EXTENSION_NAME]);

            let mut create_info = vk::InstanceCreateInfo::builder().application_info(&app_info);

            if use_validation {
                layers.push(VALIDATION_LAYER_NAME);
                extensions.push(DEBUG_UTILS_EXTENSION_NAME);
                create_info = create_info.push_next(&mut debug_callback_create_info);
            }

            create_info = create_info
                .enabled_layer_names(layers.as_slice())
                .enabled_extension_names(extensions.as_slice());

            match unsafe { library.create_instance(&create_info, None) } {
                Ok(instance) => instance,
                Err(err) => match err {
                    VkError(vk_error) => return Err(VulkanError::from(vk_error)),
                    LoadError(_) => {
                        unreachable!("Examination of ash's source shows this is never returned (July 31, 2021)")
                    }
                },
            }
        };

        let debug = if use_validation {
            let ut = DebugUtils::new(&library, &instance);
            let cb = unsafe { ut.create_debug_utils_messenger(&debug_callback_create_info, None) }?;
            Some(VulkanDebug { api: ut, callback: cb })
        } else {
            None
        };

        let surface_api = Surface::new(&library, &instance);
        let os_surface_api = Win32Surface::new(&library, &instance);

        let gpu = select_physical_device(&instance, &os_surface_api)?;

        let device = {
            let mut queue_create_infos = ArrayVec::<vk::DeviceQueueCreateInfo, 2>::new();
            queue_create_infos.push(
                *vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(gpu.graphics_queue_index)
                    .queue_priorities(&[1.0]),
            );

            if gpu.present_queue_index != gpu.graphics_queue_index {
                queue_create_infos.push(
                    *vk::DeviceQueueCreateInfo::builder()
                        .queue_family_index(gpu.present_queue_index)
                        .queue_priorities(&[1.0]),
                );
            }

            let features: vk::PhysicalDeviceFeatures = unsafe { std::mem::zeroed() };

            let extensions = ArrayVec::<_, 1>::from_iter([SWAPCHAIN_EXTENSION_NAME]);

            let create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(queue_create_infos.as_slice())
                .enabled_extension_names(extensions.as_slice())
                .enabled_features(&features);

            unsafe { instance.create_device(gpu.handle, &create_info, None) }?
        };

        let swapchain_api = Swapchain::new(&instance, &device);

        let present_queue = unsafe { device.get_device_queue(gpu.present_queue_index, 0) };
        let graphics_queue = unsafe { device.get_device_queue(gpu.graphics_queue_index, 0) };

        let pipeline_cache = {
            let create_info = vk::PipelineCacheCreateInfo::builder();
            unsafe { device.create_pipeline_cache(&create_info, None) }?
        };

        let fence_pool = {
            let mut pool = ArrayVec::new();

            for _ in 0..SYNC_POOL_SIZE {
                let ci = vk::FenceCreateInfo::builder();
                pool.push(unsafe { device.create_fence(&ci, None) }?);
            }

            pool
        };

        let semaphore_pool = {
            let mut pool = ArrayVec::new();

            for _ in 0..SYNC_POOL_SIZE {
                let ci = vk::SemaphoreCreateInfo::builder();
                pool.push(unsafe { device.create_semaphore(&ci, None) }?);
            }

            pool
        };

        let graphics_command_pool = {
            let create_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(gpu.graphics_queue_index)
                .flags(vk::CommandPoolCreateFlags::TRANSIENT | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

            unsafe { device.create_command_pool(&create_info, None) }?
        };

        Ok(Self {
            library,
            instance,
            gpu,
            device,
            graphics_queue,
            present_queue,
            surface_api,
            os_surface_api,
            swapchain_api,
            pipeline_cache,
            fence_pool,
            semaphore_pool,
            graphics_command_pool,
            debug,
        })
    }

    /// Fetches a fence from the context's pool, or creates a new one. If the
    /// fence needs to be signalled, a new one will be created.
    ///
    /// # Errors
    /// This function may fail for the following reasons:
    ///
    /// - `VK_ERROR_OUT_OF_HOST_MEMORY`
    /// - `VK_ERROR_OUT_OF_DEVICE_MEMORY`
    pub fn get_or_create_fence(&mut self, signalled: bool) -> Result<vk::Fence> {
        if signalled {
            let ci = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
            Ok(unsafe { self.device.create_fence(&ci, None) }?)
        } else if let Some(fence) = self.fence_pool.pop() {
            Ok(fence)
        } else {
            let ci = vk::FenceCreateInfo::builder();
            Ok(unsafe { self.device.create_fence(&ci, None) }?)
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

    /// Fetches a semaphore from the context's pool, or creates a new one.
    ///
    /// # Errors
    /// This function may fail for the following reasons:
    ///
    /// - `VK_ERROR_OUT_OF_HOST_MEMORY`
    /// - `VK_ERROR_OUT_OF_DEVICE_MEMORY`
    pub fn get_or_create_semaphore(&mut self) -> Result<vk::Semaphore> {
        if let Some(semaphore) = self.semaphore_pool.pop() {
            Ok(semaphore)
        } else {
            let ci = vk::SemaphoreCreateInfo::builder();
            Ok(unsafe { self.device.create_semaphore(&ci, None) }?)
        }
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

    pub fn create_shader(&mut self, source: &[u8]) -> Result<vk::ShaderModule> {
        if source.len() % 4 == 0 {
            let words = unsafe { std::slice::from_raw_parts(source.as_ptr().cast(), source.len() / 4) };
            let ci = vk::ShaderModuleCreateInfo::builder().code(words);

            Ok(unsafe { self.device.create_shader_module(&ci, None) }?)
        } else {
            Err(VulkanError::InvalidSpirV)
        }
    }

    pub fn destroy_shader(&mut self, shader: vk::ShaderModule) {
        unsafe {
            self.device.destroy_shader_module(shader, None);
        }
    }

    pub fn create_graphics_pipeline(&mut self, create_info: &vk::GraphicsPipelineCreateInfo) -> Result<vk::Pipeline> {
        let mut pipeline = vk::Pipeline::default();

        unsafe {
            self.device
                .fp_v1_0()
                .create_graphics_pipelines(
                    self.device.handle(),
                    self.pipeline_cache,
                    1,
                    create_info,
                    std::ptr::null(),
                    &mut pipeline,
                )
                .result()?;
        }

        Ok(pipeline)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // We're shutting down, so ignore errors
            let _ = self.device.device_wait_idle();

            for fence in &self.fence_pool {
                self.device.destroy_fence(*fence, None);
            }

            for semaphore in &self.semaphore_pool {
                self.device.destroy_semaphore(*semaphore, None);
            }

            if let Some(debug) = self.debug.as_ref() {
                debug.api.destroy_debug_utils_messenger(debug.callback, None);
            }

            self.device.destroy_pipeline_cache(self.pipeline_cache, None);

            self.device.destroy_command_pool(self.graphics_command_pool, None);

            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

unsafe extern "system" fn debug_callback(
    _severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    println!("Vulkan: {:?}", CStr::from_ptr((*callback_data).p_message));

    vk::FALSE
}

pub(crate) struct Gpu {
    pub handle: vk::PhysicalDevice,
    pub graphics_queue_index: u32,
    pub present_queue_index: u32,
}

/// # Errors
/// This function may fail for the following reasons:
///
/// - No GPU that supports both rendering and presentation could be found.
/// - `VK_ERROR_OUT_OF_HOST_MEMORY`
/// - `VK_ERROR_OUT_OF_DEVICE_MEMORY`
/// - `VK_ERROR_INITIALIZATION_FAILED`
fn select_physical_device(instance: &Instance, surface_api: &Win32Surface) -> Result<Gpu> {
    let physical_devices = load_vk_objects::<_, _, MAX_PHYSICAL_DEVICES>(|count, ptr| unsafe {
        instance
            .fp_v1_0()
            .enumerate_physical_devices(instance.handle(), count, ptr)
    })?;

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
                return Ok(Gpu {
                    handle: *physical_device,
                    graphics_queue_index: graphics_i.try_into().unwrap(),
                    present_queue_index: present_i.try_into().unwrap(),
                });
            }
        }
    }

    Err(VulkanError::NoSuitableGpu)
}

pub(crate) fn load_vk_objects<T, F, const COUNT: usize>(mut func: F) -> Result<ArrayVec<T, COUNT>>
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
