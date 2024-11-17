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

    pub struct StackTrace {
        pub fp: usize,
        pub pc_ptr: *const usize,
    }

    impl StackTrace {
        #[inline(always)]
        pub unsafe fn start() -> Option<Self> {
            let mut fp: usize;
            unsafe { core::arch::asm!("mov {}, rbp", out(reg) fp) };
            let pc_ptr = fp.checked_add(mem::size_of::<usize>())?;
            Some(Self {
                fp,
                pc_ptr: pc_ptr as *const usize,
            })
        }

        pub unsafe fn next(self) -> Option<Self> {
            let fp = unsafe { *(self.fp as *const usize) };
            let pc_ptr = fp.checked_add(mem::size_of::<usize>())?;
            Some(Self {
                fp: fp,
                pc_ptr: pc_ptr as *const usize,
            })
        }
    }

    pub fn print_trace(depth: usize) {
        let mut trace = unsafe { StackTrace::start() };
        for _ in 0..depth {
            if let Some(t) = trace {
                print_trace_entry(&t);
                trace = unsafe { t.next() };
            } else {
                break;
            }
        }
    }
    #[inline(always)]
    fn print_trace_entry(trace: &StackTrace) {
        let pc = unsafe { *trace.pc_ptr };
        sprintln!("{:x}", pc);
    }
}
