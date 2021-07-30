use std::{
    cmp::min,
    convert::TryInto,
    ffi::{c_void, CStr},
    iter::FromIterator,
    os::raw::c_char,
};

use pal::{
    vulkan::{
        vk, DebugUtils, Device, DeviceV1_0, EntryCustom, EntryV1_0, Instance, InstanceV1_0,
        LoadError, Surface, Swapchain, VkError, Win32Surface,
    },
    win32::{
        Foundation::{HINSTANCE, PSTR},
        System::{
            Diagnostics::Debug::{SetErrorMode, SEM_FAILCRITICALERRORS},
            LibraryLoader::{GetProcAddress, LoadLibraryA},
        },
    },
};

use utils::array_vec::ArrayVec;

use super::error::{RendererError, RendererResult};

const MAX_PHYSICAL_DEVICES: usize = 16;
const MAX_QUEUE_FAMILIES: usize = 64;

const VALIDATION_LAYER_NAME: *const c_char = "VK_LAYER_KHRONOS_validation\0".as_ptr().cast();
const SURFACE_EXTENSION_NAME: *const c_char = "VK_KHR_surface\0".as_ptr().cast();
const DEBUG_UTILS_EXTENSION_NAME: *const c_char = "VK_EXT_debug_utils\0\0".as_ptr().cast();
const WIN32_SURFACE_EXTENSION_NAME: *const c_char = "VK_KHR_win32_surface\0".as_ptr().cast();
const SWAPCHAIN_EXTENSION_NAME: *const c_char = "VK_KHR_swapchain\0".as_ptr().cast();

pub struct VulkanDebug {
    api: DebugUtils,
    callback: vk::DebugUtilsMessengerEXT,
}

pub struct VulkanContext {
    #[allow(dead_code)]
    library: EntryCustom<HINSTANCE>,
    instance: Instance,
    pub gpu: Gpu,
    pub device: Device,

    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,

    pub surface_api: Surface,
    pub os_surface_api: Win32Surface,
    pub swapchain_api: Swapchain,

    debug: Option<VulkanDebug>,
}

impl VulkanContext {
    pub fn new(use_validation: bool) -> RendererResult<Self> {
        let library = {
            let os_library = unsafe {
                let lib = LoadLibraryA("vulkan-1");
                if lib.is_null() {
                    return Err(RendererError::LibraryNotFound("vulkan-1"));
                }
                SetErrorMode(SEM_FAILCRITICALERRORS);
                lib
            };

            EntryCustom::new_custom(os_library, |lib, name| unsafe {
                // It is safe to use PSTR and cast to *mut u8 because the C api
                // takes the lpprocname as PCSTR
                // https://docs.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress
                GetProcAddress(*lib, PSTR(name.to_bytes_with_nul().as_ptr() as _))
                    .map_or(std::ptr::null_mut(), |p| p as *mut c_void)
            })
        };

        let mut debug_callback_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(debug_callback));

        let instance = {
            let app_info = vk::ApplicationInfo::builder().api_version(vk::API_VERSION_1_1);

            let mut layers = ArrayVec::<*const c_char, 1>::new();
            let mut extensions = ArrayVec::<*const c_char, 3>::from_iter([
                SURFACE_EXTENSION_NAME,
                WIN32_SURFACE_EXTENSION_NAME,
            ]);

            if use_validation {
                layers.push(VALIDATION_LAYER_NAME);
                extensions.push(DEBUG_UTILS_EXTENSION_NAME);
            }

            let mut create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_layer_names(layers.as_slice())
                .enabled_extension_names(extensions.as_slice());

            if use_validation {
                create_info = create_info.push_next(&mut debug_callback_create_info);
            }

            match unsafe { library.create_instance(&create_info, None) } {
                Ok(instance) => instance,
                Err(err) => match err {
                    LoadError(missing_ext_layers) => panic!(
                        "Required layers/extensions not found: {:?}",
                        missing_ext_layers
                    ),
                    VkError(vk_error) => return Err(RendererError::VulkanError(vk_error)),
                },
            }
        };

        let debug = if use_validation {
            let ut = DebugUtils::new(&library, &instance);
            let cb = unsafe { ut.create_debug_utils_messenger(&debug_callback_create_info, None) }?;
            Some(VulkanDebug {
                api: ut,
                callback: cb,
            })
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
            debug,
        })
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            if let Some(debug) = self.debug.as_ref() {
                debug
                    .api
                    .destroy_debug_utils_messenger(debug.callback, None);
            }

            self.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

impl std::ops::Deref for VulkanContext {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

#[doc(hidden)]
unsafe extern "system" fn debug_callback(
    _severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    println!("Vulkan: {:?}", CStr::from_ptr((*callback_data).p_message));

    vk::FALSE
}

#[doc(hidden)]
pub struct Gpu {
    pub handle: vk::PhysicalDevice,
    pub graphics_queue_index: u32,
    pub present_queue_index: u32,
}

#[doc(hidden)]
fn select_physical_device(instance: &Instance, surface_api: &Win32Surface) -> RendererResult<Gpu> {
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

    Err(RendererError::NoSuitableGPU)
}

pub(crate) fn load_vk_objects<T, F, const COUNT: usize>(
    mut func: F,
) -> RendererResult<ArrayVec<T, COUNT>>
where
    F: FnMut(*mut u32, *mut T) -> vk::Result,
{
    let mut count = 0;

    func(&mut count, std::ptr::null_mut()).result()?;

    let mut buffer = ArrayVec::new();
    count = min(count, buffer.capacity().try_into().unwrap());

    func(&mut count, buffer.as_mut_ptr()).result()?;
    unsafe { buffer.set_len(count as usize) };

    Ok(buffer)
}
