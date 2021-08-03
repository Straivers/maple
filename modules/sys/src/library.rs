use std::ffi::{c_void, CStr};

use crate::platform::library as platform;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    LibraryNotFound,
}

#[derive(Debug)]
pub struct Library {
    library: platform::Library,
}

impl Library {
    /// Loads the library at the given location.
    ///
    /// # Errors
    /// This function will fail if the library could not be found or otherwise
    /// accessed.
    pub fn load(path: &str) -> Result<Self, Error> {
        platform::Library::load(path).map_or(Err(Error::LibraryNotFound), |library| Ok(Self { library }))
    }

    /// Attempts to retrieve a symbol stored within the library.
    #[must_use]
    pub fn get_symbol(&self, name: &CStr) -> Option<*mut c_void> {
        self.library.get_symbol(name)
    }
}
