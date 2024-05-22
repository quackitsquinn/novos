#![no_std]
#![no_main]

use kernel::sprintln;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    sprintln!("uh oh, the code {}", _info);
    kernel::hlt_loop();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();
    sprintln!("Hello World!");
    kernel::hlt_loop();
}
