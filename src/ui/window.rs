use fnv::FnvHasher;
use std::cmp::min;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU8;
use super::platform;

const MAX_TITLE_LENGTH: usize = 256;

#[derive(Clone, Copy)]
pub struct Window {
    pub name_hash: u32,
    pub frame_last_touched: usize,
    title: [u8; MAX_TITLE_LENGTH],
    title_length: NonZeroU8,
    pub os_window: platform::Window,
}

impl Window {
    pub fn new(title: &str, name_hash: u32, frame: usize, os_window: platform::Window) -> Window {
        let length = min(title.len(), MAX_TITLE_LENGTH);
        let slice = title[0..length].as_bytes();

        let mut window = Window {
            name_hash,
            frame_last_touched: frame,
            title: [0; MAX_TITLE_LENGTH],
            title_length: NonZeroU8::new(length as u8).unwrap(),
            os_window,
        };

        window.title[0..length].copy_from_slice(slice);
        window.title_length = NonZeroU8::new(length as u8).unwrap();

        window
    }

    pub fn hash_title(title: &str) -> u32 {
        let mut hasher = FnvHasher::default();
        title.hash(&mut hasher);
        hasher.finish() as u32
    }

    pub fn get_title(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.title[0..self.title_length.get() as usize]) }
    }
}
