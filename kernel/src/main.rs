#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::arch::asm;

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
    sprintln!("Enabled interrupts");
    loop {
        terminal!().set_position(0, 0);
        println!("clk: {} sec: {}", timer::get_ticks(), timer::get_seconds());
    }
    kernel::hlt_loop();
}
