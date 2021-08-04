//! Dynamic library loading and symbol discovery.

use std::ffi::{c_void, CStr};

use crate::platform::library as platform;

#[derive(Debug, Clone)]
pub enum Error {
    LibraryNotFound(String),
    SymbolNotFound(String, String),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LibraryNotFound(name) => f.write_fmt(format_args!("The library {} could not be found", name)),
            Self::SymbolNotFound(path, name) => {
                f.write_fmt(format_args!("The symbol {} could not be found in {}", name, path))
            }
        }
    }
}

/// A dynamically loaded library
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
        platform::Library::load(path).map_or(Err(Error::LibraryNotFound(path.to_string())), |library| {
            Ok(Self { library })
        })
    }

    pub fn path(&self) -> &str {
        self.library.path()
    }

    /// Attempts to retrieve a symbol stored within the library, returns `None`
    /// if it was not found.
    #[must_use]
    pub fn get_symbol(&self, name: &CStr) -> Result<*mut c_void, Error> {
        if let Some(sym) = self.library.get_symbol(name) {
            Ok(sym)
        } else {
            Err(Error::SymbolNotFound(
                self.path().to_string(),
                name.to_string_lossy().to_string(),
            ))
        }
    }
}
