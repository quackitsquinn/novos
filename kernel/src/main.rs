#![no_std]
#![no_main]

use core::arch::naked_asm;

extern crate alloc;

#[panic_handler]
fn panic(pi: &core::panic::PanicInfo) -> ! {
    kernel::panic::panic(pi);
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
/// Nova's entry point. This should almost *never* be either called directly or modified.
/// This function's only purpose is to call the kernel's initialization function, which will fully take over the system.
/// This function is naked because we want to make sure there is no stack manipulation before we call into the kernel.
pub extern "sysv64" fn _start() -> ! {
    naked_asm! {
        "mov rdi, rsp", // Move the stack pointer into the first argument register
        "call init_kernel",
    }
}
