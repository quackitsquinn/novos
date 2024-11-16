#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::{arch::asm, hint::black_box, ptr};

use kernel::sprintln;
use limine::request::{KernelAddressRequest, KernelFileRequest};
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
/// Recurses `n` times, then prints the stack trace. This is used to test the stack trace printing.
#[inline(never)]
extern "C" fn recurse(n: u32) {
    if n == 0 {
        sprintln!("Printing stack trace");
        print_trace();
        sprintln!(".. Finished printing stack trace");
        return;
    }
    recurse(n - 1);
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StackFrame {
    pub rbp: *const StackFrame,
    pub rip: *const (),
}

pub fn print_trace() {
    let mut rbp: *const StackFrame;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }
    while !rbp.is_null() {
        let frame = unsafe { ptr::read_unaligned(rbp) };
        sprintln!("{:#?}", frame);
        rbp = frame.rbp;
    }
}
