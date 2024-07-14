use limine::request::FramebufferRequest;
use spin::Mutex;

mod character;
pub mod color;
mod framebuffer;
mod screen_char;
pub mod terminal;

pub use character::get_char;

use crate::sprintln;

pub static LIMINE_FRAMEBUFFERS: FramebufferRequest = FramebufferRequest::new();

lazy_static::lazy_static! {
    pub static ref FRAMEBUFFER: Mutex<framebuffer::Framebuffer> = {
        Mutex::new(framebuffer::Framebuffer::new(&LIMINE_FRAMEBUFFERS.get_response().unwrap().framebuffers().next().unwrap()))
    };

    pub static ref TERMINAL: Mutex<terminal::Terminal> = {
        sprintln!("Initializing terminal");
        let term = Mutex::new(terminal::Terminal::new());
        sprintln!("Terminal initialized");
        term
    };
}
