#![no_main]
#![no_std]

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    // This is the entry point for the trampoline.
    loop {}
}
