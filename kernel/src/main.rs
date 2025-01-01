#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

#[panic_handler]
fn panic(pi: &core::panic::PanicInfo) -> ! {
    kernel::panic::panic(pi);
}

#[unsafe(no_mangle)]
#[cfg(not(test))]
pub extern "C" fn _start() -> ! {
    use kernel::println;

    kernel::init_kernel();

    println!("Hello, world!");
    println!("Welcome to NovOS!");

    kernel::hlt_loop()
}
