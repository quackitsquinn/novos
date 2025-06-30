#![no_main]
#![no_std]

pub mod serial;

/// Kernel load & jump routine.
/// This function is called by the bootloader to load the kernel and jump to it.
/// This does not function as an actual trampoline, so please do not jump on it.
pub fn jump() -> ! {
    serial::init();
    loop {}
}
