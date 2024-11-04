#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;


use kernel::{
    memory::{self},
    println, sprintln,
};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    sprintln!("uh oh, the code {}", _info);
    if kernel::display_init() {
        println!("uh oh, the code {}", _info);
    }
    kernel::hlt_loop();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn _start() -> ! {
    use kernel::memory::allocator;

    kernel::init_kernel();

    sprintln!("Initialized kernel");
    x86_64::instructions::interrupts::enable();
    sprintln!("Enabled interrupts");
    fn fnn() {
        fnn();
    }
    loop {
        fnn();
        assert!(allocator::get_allocation_balance() == 0);
    }
    kernel::hlt_loop();
    memory::allocator::output_blocks();
}
