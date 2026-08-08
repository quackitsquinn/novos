use core::{
    arch::asm,
    fmt::{Debug, Write},
};

use arrayvec::ArrayVec;

pub mod sym;

#[derive(Debug, Clone, Copy)]
pub struct StackFrame {
    pub instruction_pointer: *const (),
    pub last_frame: *mut (),
}

/// A stack trace, consisting of a fixed-size array of stack frames.
pub struct StackTrace<const LIMIT: usize> {
    frames: ArrayVec<StackFrame, LIMIT>,
}

impl<const LIMIT: usize> Debug for StackTrace<LIMIT> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "StackTrace{:?}", &*self.frames)
    }
}

impl<const LIMIT: usize> StackTrace<LIMIT> {
    /// Prints the stack trace to the given writer, skipping the first `skip_levels` frames.
    pub fn print(&self, skip_levels: usize, writer: impl Write) {
        let mut writer = writer;

        for (i, frame) in self.frames.iter().enumerate() {
            if i < skip_levels {
                continue;
            }

            let addr = frame.instruction_pointer;
            let symbol = sym::resolve_sym_demangle(addr);

            match symbol {
                Some(sym) => {
                    writeln!(writer, "{} {:p}: {}", i - skip_levels, addr, sym).unwrap();
                }
                None => {
                    writeln!(writer, "{} {:p}: <unknown>", i - skip_levels, addr).unwrap();
                }
            }
        }
    }
}
/// Returns a stack trace of the current call stack, up to a maximum of `LIMIT` frames.
#[inline(never)]
pub fn collect_stacktrace<const LIMIT: usize>() -> StackTrace<LIMIT> {
    let mut frames = ArrayVec::new();
    let mut current_frame = match read_caller_frame(1) {
        None => return StackTrace { frames },
        Some(rip) => rip.last_frame,
    };

    while !current_frame.is_null() && frames.len() < LIMIT {
        if let Some(frame) = unsafe { read_frame(current_frame) } {
            frames.push(frame);
            current_frame = frame.last_frame;
        } else {
            break;
        }
    }

    StackTrace { frames }
}

/// Returns the instruction pointer of the caller at the given level in the call stack.
/// Level 0 is the immediate caller, level 1 is the caller's caller, and so on. Returns `None` if the level is out of bounds.
#[inline(never)]
pub fn read_caller_frame(level: usize) -> Option<StackFrame> {
    let mut curr = root_frame();
    for _ in 0..level + 1 {
        if let Some(frame) = unsafe { read_frame(curr) } {
            curr = frame.last_frame;
        } else {
            return None;
        }
    }
    unsafe { read_frame(curr) }
}

#[inline(never)]
fn root_frame() -> *const () {
    let mut rbp: *const ();
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
        // We want the caller's frame, not ours. Traverse a frame up.
        read_frame(rbp).map_or(core::ptr::null(), |frame| frame.last_frame)
    }
}

unsafe fn read_frame(frame: *const ()) -> Option<StackFrame> {
    #[derive(Clone, Copy)]
    #[repr(C)]
    struct X86StackFrame {
        rbp: *const X86StackFrame,
        rip: *const (),
    }

    if frame.is_null() {
        return None;
    }

    let frame = &unsafe { *(frame as *const X86StackFrame) };

    Some(StackFrame {
        instruction_pointer: frame.rip,
        last_frame: frame.rbp as *mut (),
    })
}
