use limine::request::FramebufferRequest;
use spin::Mutex;

mod character;
pub mod color;
mod framebuffer;

pub use character::get_char;

pub static LIMINE_FRAMEBUFFERS: FramebufferRequest = FramebufferRequest::new();

lazy_static::lazy_static! {
    pub static ref FRAMEBUFFER: Mutex<framebuffer::Framebuffer> = {
        Mutex::new(framebuffer::Framebuffer::new(&LIMINE_FRAMEBUFFERS.get_response().unwrap().framebuffers().next().unwrap()))
    };
}
