#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::{arch::asm, hint::black_box};

use kernel::sprintln;
use log::{error, log_enabled, trace};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    sprintln!("uh oh, the code {}", _info);
    print_trace();
    sprintln!(".. Finished printing stack trace");
    kernel::hlt_loop();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::init_kernel();
    #[cfg(test)]
    test_main();
    recurse(10);
    kernel::hlt_loop();
}
/// Recurses `n` times, then panics. Current use case is to test the stack trace printing.
fn recurse(n: u32) {
    if n == 0 {
        panic!("done");
    }
    recurse(n - 1);
    black_box(n);
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StackFrame {
    pub rbp: *const StackFrame,
    pub rip: usize,
}

pub fn print_trace() {
    let mut rbp: *const StackFrame;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }

    while !rbp.is_null() {
        let frame = unsafe { *rbp };
        sprintln!("{:x}", frame.rip);
        rbp = frame.rbp;
    }
}
