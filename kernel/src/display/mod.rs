use framebuffer::Framebuffer;
use limine::request::FramebufferRequest;

mod character;
pub mod color;
mod framebuffer;
mod screen_char;
pub mod terminal;

pub use character::get_char;

use crate::{sprintln, util::OnceMutex};

pub static LIMINE_FRAMEBUFFERS: FramebufferRequest = FramebufferRequest::new();

pub static FRAMEBUFFER: OnceMutex<Framebuffer> = OnceMutex::new();
pub static TERMINAL: OnceMutex<terminal::Terminal> = OnceMutex::new();

pub fn init() {
    sprintln!("Creating framebuffer");
    FRAMEBUFFER.init(Framebuffer::new(
        &LIMINE_FRAMEBUFFERS
            .get_response()
            .unwrap()
            .framebuffers()
            .next()
            .unwrap(),
    ));
    sprintln!("Framebuffer initialized.. Creating terminal");
    TERMINAL.init(terminal::Terminal::new());
    sprintln!("Terminal initialized");
}

// Gets the global terminal instance.
#[macro_export]
macro_rules! terminal {
    () => {
        $crate::display::TERMINAL.get()
    };
}

// Gets the global framebuffer instance.
#[macro_export]
macro_rules! framebuffer {
    () => {
        $crate::display::FRAMEBUFFER.get()
    };
}
