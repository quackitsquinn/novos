#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![reexport_test_harness_main = "test_main"]

use core::hint::black_box;

use kernel::sprintln;
use log::{error, log_enabled, trace};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    sprintln!("uh oh, the code {}", _info);
    kernel::hlt_loop();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn _start() -> ! {
    use kernel::hlt_loop;

    kernel::init_kernel();
    hlt_loop()
}

#[no_mangle]
#[cfg(test)]
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();
    test_main();
    kernel::hlt_loop();
}
