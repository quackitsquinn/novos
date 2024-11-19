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
    

    kernel::init_kernel();

    kernel::hlt_loop()
}
