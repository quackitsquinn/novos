//! Display output.
use core::convert::Infallible;

use cake::OnceMutex;
use framebuffer::Framebuffer;

pub mod character;
pub mod color;
mod framebuffer;
pub mod req_data;
pub mod screen_char;
pub mod terminal;

pub use character::get_char;

use crate::{declare_module, requests};

/// The global framebuffer instance.
pub static FRAMEBUFFER: OnceMutex<Framebuffer> = OnceMutex::uninitialized();
/// The global terminal instance.
pub static TERMINAL: OnceMutex<terminal::Terminal> = OnceMutex::uninitialized();

declare_module!("display", init);

fn init() -> Result<(), Infallible> {
    FRAMEBUFFER.call_init(|| unsafe { Framebuffer::new(requests::FRAMEBUFFER.get()) });
    TERMINAL.call_init(|| terminal::Terminal::new(1, 2));
    Ok(())
}

/// Gets the global terminal instance.
#[macro_export]
macro_rules! terminal {
    () => {
        $crate::display::TERMINAL.get()
    };
}

/// Gets the global framebuffer instance.
#[macro_export]
macro_rules! framebuffer {
    () => {
        $crate::display::FRAMEBUFFER.get()
    };
}
