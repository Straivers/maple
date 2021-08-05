use ash::vk;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    WindowNotValid,
    InternalError(Box<dyn std::error::Error>),
}

#[doc(hidden)]
impl From<vulkan_utils::Error> for Error {
    fn from(vkr: vulkan_utils::Error) -> Self {
        Error::InternalError(Box::new(vkr))
    }
}

#[doc(hidden)]
impl From<vk::Result> for Error {
    fn from(vkr: vk::Result) -> Self {
        Error::InternalError(Box::new(vulkan_utils::Error::from(vkr)))
    }
}
