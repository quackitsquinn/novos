use core::arch::asm;

use x86_64::{
    registers::rflags::RFlags,
    structures::{
        gdt::SegmentSelector,
        idt::{InterruptStackFrame, InterruptStackFrameValue},
    },
    VirtAddr,
};

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

    pub unsafe fn new(rip: VirtAddr, rsp: VirtAddr, cs: SegmentSelector) -> InterruptContext {
        let mut ctx = Self::zero();
        ctx.int_frame.instruction_pointer = rip;
        ctx.int_frame.stack_pointer = rsp;
        ctx.int_frame.code_segment = cs;
        // TODO: Check if this is correct
        ctx.int_frame.cpu_flags = RFlags::INTERRUPT_FLAG | RFlags::IOPL_LOW | RFlags::IOPL_HIGH;
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

    pub unsafe fn load(&self, old: &mut Self) {
        unsafe {
            asm! {
                ""
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct InterruptCodeContext {
    pub context: Context,
    pub code: u64,
    pub int_frame: InterruptStackFrameValue,
}

impl InterruptCodeContext {
    pub const fn zero() -> InterruptCodeContext {
        unsafe { core::mem::zeroed() }
    }

    pub unsafe fn zero_with_frame(frame: InterruptStackFrame, code: u64) -> InterruptCodeContext {
        let mut ctx = Self::zero();
        ctx.int_frame = *frame;
        ctx.code = code;
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
