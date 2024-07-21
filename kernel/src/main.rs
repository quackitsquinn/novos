#![no_std]
#![no_main]

use kernel::{
    display::{self, color::Color, terminal},
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
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();
    sprintln!("Hello, World!");
    println!("Hello, World!");
    kernel::hlt_loop();
}
