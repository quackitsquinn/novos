//! Persistent framebuffer information.
use core::fmt::Debug;

use cake::Mutex;
use cake::{LimineData, limine::response::FramebufferResponse};

/// Information about the framebuffer.
#[derive(Debug)]
pub struct FramebufferInfo {
    /// The width of the framebuffer.
    pub width: u64,
    /// The height of the framebuffer.
    pub height: u64,
    /// The pitch (e.g. the number of bytes in a row) of the framebuffer.
    pub pitch: u64,
    /// The number of **bits** (*not bytes*) per pixel.
    pub bpp: u16,
    ptr: Mutex<(bool, *mut u8)>,
}

impl FramebufferInfo {
    /// Creates a new `FramebufferInfo` from the given Limine framebuffer response.
    pub fn new(resp: LimineData<'_, FramebufferResponse>) -> Self {
        let framebuffer = resp.framebuffers().nth(0).expect("No framebuffer found");
        Self {
            width: framebuffer.width(),
            height: framebuffer.height(),
            pitch: framebuffer.pitch(),
            bpp: framebuffer.bpp(),
            ptr: Mutex::new((false, framebuffer.addr() as *mut u8)),
        }
    }

    /// Update the inner pointer with a new value.
    pub unsafe fn update_ptr(&self, ptr: *mut u8) {
        *self.ptr.lock() = (true, ptr);
    }

    /// Get a raw pointer to the framebuffer.
    pub fn ptr(&self) -> *mut u8 {
        let s = self.ptr.lock();

        if !s.0 {
            panic!("Framebuffer pointer not initialized");
        }

        s.1
    }

    /// Get a raw pointer to the framebuffer without checking if it's initialized.
    /// # Safety
    /// The caller must ensure that the pointer is still a valid pointer to the framebuffer.
    pub unsafe fn ptr_unchecked(&self) -> *mut u8 {
        self.ptr.lock().1
    }
}

unsafe impl Send for FramebufferInfo {}
unsafe impl Sync for FramebufferInfo {}
