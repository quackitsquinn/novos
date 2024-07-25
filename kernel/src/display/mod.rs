use framebuffer::Framebuffer;
use limine::request::FramebufferRequest;
use spin::{Mutex, Once};

mod character;
pub mod color;
mod framebuffer;
mod screen_char;
pub mod terminal;

pub use character::get_char;

use crate::sprintln;

pub static LIMINE_FRAMEBUFFERS: FramebufferRequest = FramebufferRequest::new();

pub static FRAMEBUFFER: Once<Mutex<Framebuffer>> = Once::new();
pub static TERMINAL: Once<Mutex<terminal::Terminal>> = Once::new();

pub fn init() {
    sprintln!("Creating framebuffer");
    FRAMEBUFFER.call_once(|| {
        Mutex::new(Framebuffer::new(
            &LIMINE_FRAMEBUFFERS
                .get_response()
                .unwrap()
                .framebuffers()
                .next()
                .unwrap(),
        ))
    });
    sprintln!("Framebuffer initialized.. Creating terminal");
    TERMINAL.call_once(|| Mutex::new(terminal::Terminal::new()));
    sprintln!("Terminal initialized");
}

// Gets the global terminal instance.
#[macro_export]
macro_rules! terminal {
    () => {
        $crate::display::TERMINAL.get().unwrap().lock()
    };
}

// Gets the global framebuffer instance.
#[macro_export]
macro_rules! framebuffer {
    () => {
        $crate::display::FRAMEBUFFER.get().unwrap().lock()
    };
}
