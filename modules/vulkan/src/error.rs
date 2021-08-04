use ash::vk;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// `VkResult` values that represent an error (<0)
pub enum Error {
    /// An unknown error was encountered
    Unknown,
    /// Vulkan 1.0: Could not find a GPU that supports both graphics and presentation
    NoSuitableGpu,
    /// Vulkan 1.0: Vulkan ran out of system RAM
    OutOfHostMemory,
    /// Vulkan 1.1: A pool memory allocation has failed
    OutOfPoolMemory,
    /// Vulkan 1.0: The GPU ran out of memory
    OutOfDeviceMemory,
    /// Vulkan 1.0: Object initialization failed
    InitializationFailed,
    /// Vulkan 1.0: The Vulkan driver crashed
    DriverCrashed,
    /// Vulkan 1.0: An attempt to map VRAM failed
    MemoryMapFailed,
    /// Vulkan 1.0: A requested instance or device layer could not be found
    LayerNotPresent,
    /// Vulkan 1.0: A requested instance or device extension could not be found
    ExtensionNotPresent,
    /// Vulkan 1.0: A requested device feature is not supported on this device
    FeatureNotPresent,
    /// Vulkan 1.0: The driver does not support the requested version of Vulkan
    IncompatibleDriver,
    /// Vulkan 1.0: Too many objects of a particular type were created
    TooManyObjects,
    /// Vulkan 1.0: The image format is not supported on the device
    FormatNotSupported,
    /// Vulkan 1.2: A pool allocation failed due to fragmentation
    DescriptorPoolTooFragmented,
    /// Vulkan 1.1: Attempted to use a handle that is of incorrect
    InvalidExternalHandle,
    /// Vulkan 1.2: A buffer creation or memory allocation operation failed
    /// because the requested address is not available. Or, a shader group
    /// invocation failed because the group is no longer valid.
    InvalidOpaqueCaptureAddress,
    /// VK_KHR_surface: Attempted to access a surface whose window was destroyed
    SurfaceLost,
    /// VK_KHR_surface: The window is already in use by another swapchain, or is
    /// claimed by another API (DXGI, OGL, etc.)
    NativeWindowInUse,
    /// VK_KHR_swapchain: The swapchain cannot present to the surface because
    /// the window was resized and the swapchain has not been updated.
    SwapchainOutOfDate,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self))
    }
}

impl std::error::Error for Error {}

#[doc(hidden)]
impl From<vk::Result> for Error {
    fn from(vkr: vk::Result) -> Self {
        assert!(
            vkr.as_raw() < 0,
            "VulkanError is exclusively for representing errors, not all VkResult status codes"
        );

        match vkr {
            vk::Result::ERROR_OUT_OF_HOST_MEMORY => Self::OutOfHostMemory,
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => Self::OutOfDeviceMemory,
            vk::Result::ERROR_INITIALIZATION_FAILED => Self::InitializationFailed,
            vk::Result::ERROR_DEVICE_LOST => Self::DriverCrashed,
            vk::Result::ERROR_MEMORY_MAP_FAILED => Self::MemoryMapFailed,
            vk::Result::ERROR_LAYER_NOT_PRESENT => Self::LayerNotPresent,
            vk::Result::ERROR_EXTENSION_NOT_PRESENT => Self::ExtensionNotPresent,
            vk::Result::ERROR_FEATURE_NOT_PRESENT => Self::FeatureNotPresent,
            vk::Result::ERROR_INCOMPATIBLE_DRIVER => Self::IncompatibleDriver,
            vk::Result::ERROR_TOO_MANY_OBJECTS => Self::TooManyObjects,
            vk::Result::ERROR_FORMAT_NOT_SUPPORTED => Self::FormatNotSupported,
            vk::Result::ERROR_UNKNOWN => Self::Unknown,

            // Vulkan 1.1
            vk::Result::ERROR_OUT_OF_POOL_MEMORY => Self::OutOfPoolMemory,
            vk::Result::ERROR_INVALID_EXTERNAL_HANDLE => Self::InvalidExternalHandle,

            // Vulkan 1.2
            vk::Result::ERROR_FRAGMENTATION => Self::DescriptorPoolTooFragmented,
            vk::Result::ERROR_INVALID_OPAQUE_CAPTURE_ADDRESS => Self::InvalidOpaqueCaptureAddress,

            // VK_KHR_surface
            vk::Result::ERROR_SURFACE_LOST_KHR => Self::SurfaceLost,
            vk::Result::ERROR_NATIVE_WINDOW_IN_USE_KHR => Self::NativeWindowInUse,

            // VK_KHR_swapchain
            vk::Result::ERROR_OUT_OF_DATE_KHR => Self::SwapchainOutOfDate,

            _ => unreachable!("Unknown VkResult error"),
        }
    }
}
