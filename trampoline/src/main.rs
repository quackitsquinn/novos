#![no_main]
#![no_std]

extern crate alloc;

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    trampoline::panic(info);
}

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    trampoline::jump()
}
