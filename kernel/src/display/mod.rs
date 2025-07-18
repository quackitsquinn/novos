use core::convert::Infallible;

use cake::{declare_module, OnceMutex};
use framebuffer::Framebuffer;

mod character;
pub mod color;
mod framebuffer;
mod screen_char;
pub mod terminal;

pub use character::get_char;

use crate::requests::FRAMEBUFFERS;

pub static FRAMEBUFFER: OnceMutex<Framebuffer> = OnceMutex::uninitialized();
pub static TERMINAL: OnceMutex<terminal::Terminal> = OnceMutex::uninitialized();

declare_module!("display", init);

fn init() -> Result<(), Infallible> {
    FRAMEBUFFER.init(Framebuffer::new(
        &FRAMEBUFFERS
            .get()
            .expect("framebuffer uninit")
            .framebuffers()
            .next()
            .unwrap(),
    ));
    TERMINAL.init(terminal::Terminal::new(1, 2));
    Ok(())
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
