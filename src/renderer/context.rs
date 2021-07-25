use std::{
    cmp::min,
    ffi::{c_void, CStr},
    os::raw::c_char,
};

use pal::{
    vulkan::*,
    win32::{
        Foundation::{HINSTANCE, PSTR},
        System::{
            Diagnostics::Debug::{SetErrorMode, SEM_FAILCRITICALERRORS},
            LibraryLoader::{GetProcAddress, LoadLibraryA},
        },
    },
};

use utils::array_vec::ArrayVec;

use super::error::*;

const MAX_PHYSICAL_DEVICES: usize = 16;
const MAX_QUEUE_FAMILIES: usize = 64;

#[cfg(debug_assertions)]
const INSTANCE_LAYERS: [*const c_char; 1] = ["VK_LAYER_KHRONOS_validation\0".as_ptr().cast()];

#[cfg(not(debug_assertions))]
const INSTANCE_LAYERS: [*const c_char; 0] = [];

#[cfg(all(target_os = "windows", debug_assertions))]
const INSTANCE_EXTENSIONS: [*const c_char; 3] = [
    "VK_KHR_surface\0".as_ptr().cast(),
    "VK_KHR_win32_surface\0".as_ptr().cast(),
    "VK_EXT_debug_utils\0\0".as_ptr().cast(),
];

#[cfg(all(target_os = "windows", not(debug_assertions)))]
const INSTANCE_EXTENSIONS: [*const c_char; 2] = [
    "VK_KHR_surface\0".as_ptr().cast(),
    "VK_KHR_win32_surface\0".as_ptr().cast(),
];

pub struct VulkanContext {
    library: EntryCustom<HINSTANCE>,
    instance: Instance,
    physical_device: Gpu,
    device: Device,

    surface_api: Surface,
    os_surface_api: Win32Surface,

    #[cfg(debug_assertions)]
    debug_utils_api: DebugUtils,
    #[cfg(debug_assertions)]
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanContext {
    pub fn new(_use_validation: bool) -> Result<Self, RendererError> {
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

        #[cfg(debug_assertions)]
        let debug_callback_create_info = vk::DebugUtilsMessengerCreateInfoEXT {
            s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            p_next: std::ptr::null(),
            flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::all(),
            pfn_user_callback: Some(debug_callback),
            p_user_data: std::ptr::null_mut(),
        };

        let instance = {
            let app_info = vk::ApplicationInfo::builder().api_version(vk::API_VERSION_1_1);

            let mut create_info = vk::InstanceCreateInfo {
                s_type: vk::StructureType::INSTANCE_CREATE_INFO,
                flags: vk::InstanceCreateFlags::empty(),
                p_next: std::ptr::null(),
                p_application_info: &*app_info,
                enabled_layer_count: INSTANCE_LAYERS.len() as u32,
                pp_enabled_layer_names: INSTANCE_LAYERS.as_ptr(),
                enabled_extension_count: INSTANCE_EXTENSIONS.len() as u32,
                pp_enabled_extension_names: INSTANCE_EXTENSIONS.as_ptr(),
            };

            #[cfg(debug_assertions)]
            {
                create_info.p_next = &debug_callback_create_info as *const _ as _;
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

        #[cfg(debug_assertions)]
        let debug_utils_api = DebugUtils::new(&library, &instance);

        #[cfg(debug_assertions)]
        let debug_utils_messenger = unsafe {
            debug_utils_api.create_debug_utils_messenger(&debug_callback_create_info, None)
        }?;

        let surface_api = Surface::new(&library, &instance);
        let os_surface_api = Win32Surface::new(&library, &instance);

        let physical_device = select_physical_device(&instance, &os_surface_api)?;

        let device = {
            let queue_priority = 1.0;
            let queue_create_infos = [
                vk::DeviceQueueCreateInfo {
                    s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::DeviceQueueCreateFlags::empty(),
                    queue_family_index: physical_device.graphics_queue_index,
                    queue_count: 1,
                    p_queue_priorities: &queue_priority,
                },
                vk::DeviceQueueCreateInfo {
                    s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::DeviceQueueCreateFlags::empty(),
                    queue_family_index: physical_device.present_queue_index,
                    queue_count: 1,
                    p_queue_priorities: &queue_priority,
                },
            ];

            let count =
                if physical_device.present_queue_index == physical_device.graphics_queue_index {
                    1
                } else {
                    2
                };

            let features: vk::PhysicalDeviceFeatures = unsafe { std::mem::zeroed() };

            let create_info = vk::DeviceCreateInfo {
                s_type: vk::StructureType::DEVICE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::DeviceCreateFlags::empty(),
                queue_create_info_count: count,
                p_queue_create_infos: queue_create_infos.as_ptr(),
                enabled_layer_count: 0,
                pp_enabled_layer_names: std::ptr::null(),
                enabled_extension_count: 0,
                pp_enabled_extension_names: std::ptr::null(),
                p_enabled_features: &features,
            };

            unsafe { instance.create_device(physical_device.handle, &create_info, None) }?
        };

        Ok(Self {
            library,
            instance,
            physical_device,
            device,
            surface_api,
            os_surface_api,
            #[cfg(debug_assertions)]
            debug_utils_api,
            #[cfg(debug_assertions)]
            debug_utils_messenger,
        })
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            #[cfg(debug_assertions)]
            {
                self.debug_utils_api
                    .destroy_debug_utils_messenger(self.debug_utils_messenger, None)
            }

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

struct Gpu {
    handle: vk::PhysicalDevice,
    graphics_queue_index: u32,
    present_queue_index: u32,
}

fn select_physical_device(
    instance: &Instance,
    surface_api: &Win32Surface,
) -> Result<Gpu, RendererError> {
    for physical_device in &load_physical_devices(instance)? {
        let queue_families = load_queue_families(instance, *physical_device);

        let mut graphics = None;
        let mut present = None;
        for (queue_family_index, properties) in queue_families.iter().enumerate() {
            if properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics = Some(queue_family_index);
            }

            if unsafe {
                surface_api.get_physical_device_win32_presentation_support(
                    *physical_device,
                    queue_family_index as u32,
                )
            } {
                present = Some(queue_family_index);
            }

            if let Some((graphics_i, present_i)) = graphics.zip(present) {
                return Ok(Gpu {
                    handle: *physical_device,
                    graphics_queue_index: graphics_i as u32,
                    present_queue_index: present_i as u32,
                });
            }
        }
    }

    Err(RendererError::NoSuitableGPU)
}

fn load_physical_devices<'a>(
    instance: &Instance,
) -> Result<ArrayVec<vk::PhysicalDevice, MAX_PHYSICAL_DEVICES>, RendererError> {
    let mut num_physical_devices = 0;
    unsafe {
        instance.fp_v1_0().enumerate_physical_devices(
            instance.handle(),
            &mut num_physical_devices,
            std::ptr::null_mut(),
        )
    }
    .result()?;

    let mut buffer = ArrayVec::new();
    num_physical_devices = min(num_physical_devices, buffer.capacity() as u32);

    unsafe {
        instance
            .fp_v1_0()
            .enumerate_physical_devices(
                instance.handle(),
                &mut num_physical_devices,
                buffer.as_mut_ptr_unchecked(),
            )
            .result()?;

        buffer.set_len(num_physical_devices as usize);
    }

    Ok(buffer)
}

fn load_queue_families<'a>(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> ArrayVec<vk::QueueFamilyProperties, MAX_QUEUE_FAMILIES> {
    let mut num_queue_families = 0;

    unsafe {
        instance
            .fp_v1_0()
            .get_physical_device_queue_family_properties(
                physical_device,
                &mut num_queue_families,
                std::ptr::null_mut(),
            )
    };

    let mut buffer = ArrayVec::new();
    num_queue_families = min(num_queue_families, buffer.capacity() as u32);

    unsafe {
        instance
            .fp_v1_0()
            .get_physical_device_queue_family_properties(
                physical_device,
                &mut num_queue_families,
                buffer.as_mut_ptr_unchecked(),
            );

        buffer.set_len(num_queue_families as usize);
    }

    buffer
}
