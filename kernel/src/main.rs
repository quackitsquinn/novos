#![no_std]
#![no_main]

extern crate alloc;

#[panic_handler]
fn panic(pi: &core::panic::PanicInfo) -> ! {
    kernel::panic::panic(pi);
}

#[unsafe(no_mangle)]
#[cfg(not(test))]
/// Nova's entry point. This should almost *never* be either called directly or modified.
/// This function's only purpose is to call the kernel's initialization function, which will fully take over the system.
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();
}
