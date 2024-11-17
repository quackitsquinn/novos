use core::{arch::asm, error, panic::PanicInfo};

use log::error;

use crate::{sprint, sprintln};

pub fn panic_basic(pi: &PanicInfo) {
    error!("PANIC {}", pi);
}

/// A more traditional panic handler that includes more information.
pub fn panic_extended_info(pi: &PanicInfo) -> ! {
    error!("PANIC at ");
    write_location(pi);
    sprintln!();
    error!("{}", pi.message());
    sprintln!("Backtrace:");
    stacktrace::print_trace(10);
    sprintln!("... only traversed 10 frames");
    loop {}
}

fn write_location(pi: &PanicInfo) {
    if let Some(location) = pi.location() {
        sprint!("{}:{}", location.file(), location.line())
    } else {
        sprint!("Unknown location")
    }
}

pub fn panic(pi: &PanicInfo) -> ! {
    //panic_basic(pi);
    panic_extended_info(pi);
    loop {}
}

mod stacktrace {
    use core::{arch::asm, ffi::c_void, mem};

    use crate::sprintln;
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct StackFrame {
        pub rbp: *const StackFrame,
        pub rip: usize,
    }

    pub fn print_trace(depth: usize) {
        let mut rbp: *const StackFrame;
        unsafe {
            asm!("mov {}, rbp", out(reg) rbp);
        }
        let mut i = 0;
        while !rbp.is_null() && i < depth {
            let frame = unsafe { *rbp };
            sprintln!("{:x}", frame.rip);
            rbp = frame.rbp;
            i += 1;
        }
    }
}
