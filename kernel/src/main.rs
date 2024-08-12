#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::{alloc, arch::asm};

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
    // Write at a arbitrary position in the defined heap
    unsafe { ((kernel::memory::HEAP_MEM_OFFSET + 0x2000) as *mut u8).write_volatile(0x42) };
    // loop {
    //     terminal!().set_position(0, 0);
    //     println!("clk: {} sec: {}", timer::get_ticks(), timer::get_seconds());
    // }
    kernel::hlt_loop();
}
