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
    println!("This is a very long line. This will test that the framebuffer SHOULD NOT, I repeat SHOULD NOT crash. blah blah blah blah blah blah blah");

    kernel::hlt_loop()
}
