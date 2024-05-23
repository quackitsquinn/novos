use limine::framebuffer::Framebuffer as LimineFramebuffer;

use crate::sprintln;

use super::color::Color;

pub struct Framebuffer {
    width: usize,
    height: usize,
    pitch: usize,
    /// Bytes per pixel.
    bpp: u16,
    // We could of used a 2D array, but because of pitch we can't.
    /// The frame buffer.
    buffer: &'static mut [u8],
}

impl Framebuffer {
    pub fn new(fb: &LimineFramebuffer) -> Framebuffer {
        if fb.bpp() % 8 != 0 {
            panic!("Non-byte aligned framebuffers are not supported.");
        } else if fb.bpp() / 8 < 3 {
            panic!("Framebuffers with less than 3 bytes per pixel are not supported.");
        }

        sprintln!(
            "Framebuffer: {}x{} {}bpp ({})",
            fb.width(),
            fb.height(),
            fb.bpp(),
            fb.pitch() * fb.height()
        );
        Self {
            width: fb.width() as usize,
            height: fb.height() as usize,
            pitch: fb.pitch() as usize,
            bpp: (fb.bpp() / 8),
            buffer: unsafe {
                // Safety: We calculate the buffer size based on the pitch and height of the framebuffer.
                core::slice::from_raw_parts_mut(
                    fb.addr() as *mut u8,
                    fb.pitch() as usize * fb.height() as usize,
                )
            },
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pitch(&self) -> usize {
        self.pitch
    }

    pub fn buffer(&mut self) -> &mut [u8] {
        self.buffer
    }

    #[inline]
    pub fn set_px(&mut self, x: usize, y: usize, color: Color) {
        assert!(x < self.width && y < self.height, "Pixel out of bounds");
        let offset = (y * self.pitch) + (x * (self.bpp as usize));
        color.to_slice(&mut self.buffer[offset..offset + self.bpp as usize]);
    }
}

unsafe impl Send for Framebuffer {}
