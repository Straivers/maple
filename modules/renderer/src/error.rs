use std::num::NonZeroI32;

use ash::vk;

pub type RendererResult<T> = Result<T, RendererError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererError {
    LibraryNotFound(&'static str),
    VulkanError(VulkanError),
    NoSuitableGPU,
}

#[doc(hidden)]
impl From<vk::Result> for RendererError {
    fn from(vkr: vk::Result) -> Self {
        RendererError::VulkanError(VulkanError::from(vkr))
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// VkResult values that represent an error (<0)
pub struct VulkanError(NonZeroI32);

macro_rules! vk_error {
    ($name:ident, $x:expr, $doc_string:literal) => {
        // Safety: This should be `NonZeroI32::new($x).unwrap()`, but `const fn
        // unwrap()` isn't stable yet.
        #[allow(dead_code)]
        #[doc = $doc_string]
        pub const $name: VulkanError = VulkanError(unsafe { NonZeroI32::new_unchecked($x) });
    };
}

#[rustfmt::skip]
impl VulkanError {
    // Vulkan 1.0
    vk_error!(OUT_OF_HOST_MEMORY, -1, "A host memory allocation has failed.");
    vk_error!(OUT_OF_DEVICE_MEMORY, -2, " A device memory allocation has failed.");
    vk_error!(INITIALIZATION_FAILED, -3, "nitialization of an object could not be completed for implementation-specific reasons.");
    vk_error!(DEVICE_LOST, -4, "The logical or physical device has been lost.");
    vk_error!(MEMORY_MAP_FAILED, -5, "Mapping of a memory object has failed.");
    vk_error!(LAYER_NOT_PRESENT, -6, "A requested layer is not present or could not be loaded.");
    vk_error!(EXTENSION_NOT_PRESENT, -7, "A requested extension is not supported.");
    vk_error!(FEATURE_NOT_PRESENT, -8, "A requested feature is not supported.");
    vk_error!(INCOMPATIBLE_DRIVER, -9, "The requested version of Vulkan is not supported by the driver or is otherwise incompatible for implementation-specific reasons.");
    vk_error!(TOO_MANY_OBJECTS, -10, "Too many objects of the type have already been created.");
    vk_error!(FORMAT_NOT_SUPPORTED, -11, "A requested format is not supported on this device.");
    vk_error!(FRAGMENTED_POOL, -12, "A pool allocation has failed due to fragmentation of the pool’s memory.");
    vk_error!(UNKNOWN, -13, "An unknown error has occurred; either the application has provided invalid input, or an implementation failure has occurred.");

    // Vulkan 1.1
    vk_error!(OUT_OF_POOL_MEMORY, -1000069000, "A pool memory allocation has failed.");
    vk_error!(INVALID_EXTERNAL_HANDLE, -1000072003, "An external handle is not a valid handle of the specified type.");

    // Vulkan 1.2
    vk_error!(FRAGMENTATION, -1000161000, "A descriptor pool creation has failed due to fragmentation.");
    vk_error!(INVALID_OPAQUE_CAPTURE_ADDRESS, -1000257000, "A buffer creation or memory allocation failed because the requested address is not available. A shader group handle assignment failed because the requested shader group handle information is no longer valid.");

    // VK_KHR_surface
    vk_error!(SURFACE_LOST_KHR, -1000000000, "A surface is no longer available.");
    vk_error!(NATIVE_WINDOW_IN_USE_KHR, -1000000001, "The requested window is already in use by Vulkan or another API in a manner which prevents it from being used again.");

    // VK_KHR_swapchain
    vk_error!(OUT_OF_DATE_KHR, -1000001004, " A surface has changed in such a way that it is no longer compatible with the swapchain, and further presentation requests using the swapchain will fail.");
}

#[doc(hidden)]
impl From<vk::Result> for VulkanError {
    fn from(vkr: vk::Result) -> Self {
        assert!(
            vkr.as_raw() < 0,
            "VulkanError is exclusively for representing errors, not all VkResult status codes"
        );

        match vkr {
            vk::Result::ERROR_OUT_OF_HOST_MEMORY => Self::OUT_OF_HOST_MEMORY,
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => Self::OUT_OF_DEVICE_MEMORY,
            vk::Result::ERROR_INITIALIZATION_FAILED => Self::INITIALIZATION_FAILED,
            vk::Result::ERROR_DEVICE_LOST => Self::DEVICE_LOST,
            vk::Result::ERROR_MEMORY_MAP_FAILED => Self::MEMORY_MAP_FAILED,
            vk::Result::ERROR_LAYER_NOT_PRESENT => Self::LAYER_NOT_PRESENT,
            vk::Result::ERROR_EXTENSION_NOT_PRESENT => Self::EXTENSION_NOT_PRESENT,
            vk::Result::ERROR_FEATURE_NOT_PRESENT => Self::FEATURE_NOT_PRESENT,
            vk::Result::ERROR_INCOMPATIBLE_DRIVER => Self::INCOMPATIBLE_DRIVER,
            vk::Result::ERROR_TOO_MANY_OBJECTS => Self::TOO_MANY_OBJECTS,
            vk::Result::ERROR_FORMAT_NOT_SUPPORTED => Self::FORMAT_NOT_SUPPORTED,
            vk::Result::ERROR_FRAGMENTED_POOL => Self::FRAGMENTED_POOL,
            vk::Result::ERROR_UNKNOWN => Self::UNKNOWN,

            // Vulkan 1.1
            vk::Result::ERROR_OUT_OF_POOL_MEMORY => Self::OUT_OF_POOL_MEMORY,
            vk::Result::ERROR_INVALID_EXTERNAL_HANDLE => Self::INVALID_EXTERNAL_HANDLE,

            // Vulkan 1.2
            vk::Result::ERROR_FRAGMENTATION => Self::FRAGMENTATION,
            vk::Result::ERROR_INVALID_OPAQUE_CAPTURE_ADDRESS => Self::INVALID_OPAQUE_CAPTURE_ADDRESS,

            // VK_KHR_surface
            vk::Result::ERROR_SURFACE_LOST_KHR => Self::SURFACE_LOST_KHR,
            vk::Result::ERROR_NATIVE_WINDOW_IN_USE_KHR => Self::NATIVE_WINDOW_IN_USE_KHR,

            // VK_KHR_swapchain
            vk::Result::ERROR_OUT_OF_DATE_KHR => Self::OUT_OF_DATE_KHR,

            _ => unreachable!("Unknown VkResult error"),
        }
    }
}
