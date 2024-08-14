#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use kernel::{
    display::{self, color::Color, terminal},
    interrupts::hardware::timer,
    println, sprintln, terminal,
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
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();
    sprintln!("Initialized kernel");
    x86_64::instructions::interrupts::enable();
    alloc::vec![0; 100];
    sprintln!("Enabled interrupts");
    kernel::hlt_loop();
}
