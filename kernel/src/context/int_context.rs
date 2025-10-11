use x86_64::{
    registers::rflags::RFlags,
    structures::{
        gdt::SegmentSelector,
        idt::{InterruptStackFrame, InterruptStackFrameValue},
    },
    VirtAddr,
};

use super::ContextValue;

/// A representation of the context of an interrupt.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptContextValue {
    /// The general CPU context at the time of the interrupt.
    pub context: ContextValue,
    /// The interrupt stack frame at the time of the interrupt.
    pub int_frame: InterruptStackFrameValue,
}

impl InterruptContextValue {
    /// Returns a zeroed interrupt context value.
    pub const fn zero() -> InterruptContextValue {
        unsafe { core::mem::zeroed() }
    }

    /// Creates a zeroed interrupt context value with the given interrupt stack frame.
    /// # Safety
    /// The caller must ensure that the provided `frame` is valid and properly initialized.
    pub unsafe fn zero_with_frame(frame: InterruptStackFrame) -> InterruptContextValue {
        let mut ctx = Self::zero();
        ctx.int_frame = *frame;
        ctx
    }

    /// Creates a new interrupt context value.
    /// # Safety
    /// The caller must ensure that the provided `rip` and `rsp` are valid virtual addresses and that `cs` is a valid code segment selector.
    pub unsafe fn new(rip: VirtAddr, rsp: VirtAddr, cs: SegmentSelector) -> InterruptContextValue {
        let mut ctx = Self::zero();
        ctx.int_frame.instruction_pointer = rip;
        ctx.int_frame.stack_pointer = rsp;
        ctx.int_frame.code_segment = cs;
        // TODO: Check if this is correct
        ctx.int_frame.cpu_flags = RFlags::INTERRUPT_FLAG | RFlags::IOPL_LOW | RFlags::IOPL_HIGH;
        ctx
    }

    /// Switches the current context with another. Replaces `to` with the current context.
    /// # Safety
    /// The caller must ensure that the context switch is valid and that the `to` context is properly initialized.
    pub unsafe fn switch(&mut self, to: &mut Self) {
        core::mem::swap(self, to)
    }
}

/// A representation of the context of an interrupt with an associated error code.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptCodeContextValue {
    /// The general CPU context at the time of the interrupt.
    pub context: ContextValue,
    /// The associated error code.
    pub code: u64,
    /// The interrupt stack frame at the time of the interrupt.
    pub int_frame: InterruptStackFrameValue,
}

impl InterruptCodeContextValue {
    /// Zeros the interrupt code context value.
    pub const fn zero() -> InterruptCodeContextValue {
        unsafe { core::mem::zeroed() }
    }

    /// Creates a zeroed interrupt code context value with the given interrupt stack frame and error code.
    ///
    /// # Safety
    /// The caller must ensure that the provided `frame` and `code` are valid and properly initialized.
    pub unsafe fn zero_with_frame(
        frame: InterruptStackFrame,
        code: u64,
    ) -> InterruptCodeContextValue {
        let mut ctx = Self::zero();
        ctx.int_frame = *frame;
        ctx.code = code;
        ctx
    }

    /// Switches the current context with another. Replaces `to` with the current context.
    pub unsafe fn switch(&mut self, to: &mut Self) {
        core::mem::swap(self, to)
    }
}
