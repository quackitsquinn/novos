#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]

use cake::{error, info};

mod arch;
mod idt;
mod mem;
mod requests;
mod serial;

pub const STACK_SIZE: usize = 0x100_000;

/// Kernel load & jump routine.
/// This function is called by the bootloader to load the kernel and jump to it.
/// This does not function as an actual trampoline, so please do not jump on it.
pub fn jump() -> ! {
    serial::init();
    requests::load();
    info!("Loaded requests...");
    idt::load();
    let kernel = mem::init();
    info!("Kernel info: {:?}", kernel);
    info!("Loading jump point...");
    arch::load_jump_point();
    info!("Jumping to kernel...");
    unsafe { arch::jump(kernel) };
}

pub fn panic(info: &core::panic::PanicInfo) -> ! {
    if serial::is_init() {
        error!(
            "PANIC at {}: {}",
            info.location()
                .unwrap_or_else(|| core::panic::Location::caller()),
            info.message()
        );

        error!("Stack trace:");
        backtrace::trace();
    }
    // If serial is not initialized, we cannot print anything.
    // This is a very bad situation, so we just loop forever.
    loop {}
}

mod backtrace {
    use crate::println;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct StackFrame {
        pub rbp: *const StackFrame,
        pub rip: *const (),
    }

    /// Prints the current stack trace. This is a *very* basic implementation that doesn't do symbol resolution.
    pub fn trace() {
        let mut rbp: *const StackFrame;
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) rbp);
        }

        while !rbp.is_null() {
            let frame = unsafe { *rbp };
            println!("{:p}", frame.rip);
            rbp = frame.rbp;
        }
    }
}
