use x86_64::structures::idt::{InterruptStackFrame, InterruptStackFrameValue};

use super::Context;

/// A representation of the context of an interrupt.
/// Modifications to this struct *WILL* be used when the interrupt returns, so be careful.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct InterruptContext {
    pub context: Context,
    pub int_frame: InterruptStackFrameValue,
}

impl InterruptContext {
    pub const fn zero() -> InterruptContext {
        unsafe { core::mem::zeroed() }
    }

    pub unsafe fn zero_with_frame(frame: InterruptStackFrame) -> InterruptContext {
        let mut ctx = Self::zero();
        ctx.int_frame = *frame;
        ctx
    }

    pub unsafe fn switch(&mut self, to: &mut Self) -> Self {
        let mut old = Self::zero();
        unsafe {
            core::ptr::copy_nonoverlapping(self, &mut old, 1);
            core::ptr::copy_nonoverlapping(to, self, 1);
        }
        old
    }
}
