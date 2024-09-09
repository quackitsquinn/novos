#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::hint::black_box;

use kernel::{
    display::{self, color::Color, terminal},
    interrupts::hardware::timer,
    memory, println, sprintln, terminal,
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
    loop {
        create_arr_check_free();
    }
    kernel::hlt_loop();
    memory::allocator::output_blocks();
}

static mut COUNTER: u32 = 0;

fn create_arr_check_free() {
    // Make sure this doesn't get optimized out
    let mut t: alloc::vec::Vec<u32> = alloc::vec![0];
    for i in 0..100 {
        t.push(unsafe { COUNTER });
        sprintln!("Pushed {}", unsafe { COUNTER });
        unsafe {
            COUNTER += 1;
        }
    }
    black_box(t);
}
