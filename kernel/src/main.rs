#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::hint::black_box;

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

    #[cfg(test)]
    test_main();

    sprintln!("Initialized kernel");
    x86_64::instructions::interrupts::enable();
    sprintln!("Enabled interrupts");
    while true {
        create_arr_check_free();
    }
    kernel::hlt_loop();
}

fn create_arr_check_free() {
    // Make sure this doesn't get optimized out
    let t = alloc::vec![0; 10];
    black_box(t);
}
