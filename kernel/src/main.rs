#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::hint::black_box;

use kernel::{
    display::{self, color::Color, terminal},
    interrupts::hardware::timer,
    memory::{self, allocator::get_block_allocator},
    println, sprintln, terminal,
};
use log::{error, log_enabled, trace};

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
