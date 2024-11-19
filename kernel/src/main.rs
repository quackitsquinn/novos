#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use kernel::{
    memory::{self},
    println, sprintln,
};

#[panic_handler]
fn panic(pi: &core::panic::PanicInfo) -> ! {
    kernel::panic::panic(pi);
}

#[unsafe(no_mangle)]
#[cfg(not(test))]
pub extern "C" fn _start() -> ! {
    use kernel::memory::allocator;

    kernel::init_kernel();

    kernel::hlt_loop()
}
