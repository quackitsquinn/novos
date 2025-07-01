#![no_main]
#![no_std]

use log::error;

pub mod serial;

/// Kernel load & jump routine.
/// This function is called by the bootloader to load the kernel and jump to it.
/// This does not function as an actual trampoline, so please do not jump on it.
pub fn jump() -> ! {
    serial::init();
    loop {}
}

pub fn panic(info: &core::panic::PanicInfo) -> ! {
    if serial::is_init() {
        error!(
            "PANIC at {}: {}",
            info.location()
                .unwrap_or_else(|| core::panic::Location::caller()),
            info.message()
        );
    }
    // If serial is not initialized, we cannot print anything.
    // This is a very bad situation, so we just loop forever.
    loop {}
}
