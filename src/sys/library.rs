use std::ffi::{c_void, CStr};

use win32::{
    Foundation::{HINSTANCE, PSTR},
    System::{
        Diagnostics::Debug::{SetErrorMode, SEM_FAILCRITICALERRORS},
        LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
};

#[derive(Debug)]
pub struct Library {
    library: HINSTANCE,
}

impl Library {
    pub fn load(path: &str) -> Option<Self> {
        // INFO(davidzhang, Aug 1, 2021): There is an automatic conversion from
        // &str to PSTR that involves a memory allocation. However, I don't
        // expect that the application will be loading libraries willy-nilly, so
        // we should be ok.
        let library = unsafe { LoadLibraryW(path) };
        if library.is_null() {
            None
        } else {
            unsafe { SetErrorMode(SEM_FAILCRITICALERRORS) };
            Some(Self { library })
        }
    }

    pub fn get_symbol(&self, path: &CStr) -> Option<*mut c_void> {
        let symbol = unsafe { GetProcAddress(self.library, PSTR(path.to_bytes_with_nul().as_ptr() as _)) };

        symbol.map(|s| s as _)
    }
}
