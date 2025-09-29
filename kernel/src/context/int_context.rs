use core::fmt::Display;

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
/// Modifications to this struct *WILL* be used when the interrupt returns, so be careful.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptContextValue {
    pub context: ContextValue,
    pub int_frame: InterruptStackFrameValue,
}

impl InterruptContextValue {
    pub const fn zero() -> InterruptContextValue {
        unsafe { core::mem::zeroed() }
    }

    pub unsafe fn zero_with_frame(frame: InterruptStackFrame) -> InterruptContextValue {
        let mut ctx = Self::zero();
        ctx.int_frame = *frame;
        ctx
    }

    pub unsafe fn new(rip: VirtAddr, rsp: VirtAddr, cs: SegmentSelector) -> InterruptContextValue {
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
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptCodeContextValue {
    pub context: ContextValue,
    pub code: u64,
    pub int_frame: InterruptStackFrameValue,
}

impl InterruptCodeContextValue {
    pub const fn zero() -> InterruptCodeContextValue {
        unsafe { core::mem::zeroed() }
    }

    pub unsafe fn zero_with_frame(
        frame: InterruptStackFrame,
        code: u64,
    ) -> InterruptCodeContextValue {
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

impl Display for InterruptCodeContextValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // f.write_str("\n")?;
        // macro_rules! p {
        //     ($($arg:tt)*) => {
        //         f.write_fmt(format_args!($($arg)*))
        //     };
        // }
        //writeln!(f, "{}", self.context)?;
        // p!(
        //     "RIP: {:#018x} RSP: {:#018x}\n\n",
        //     self.int_frame.instruction_pointer,
        //     self.int_frame.stack_pointer
        // )?;

        // p!(
        //     "RFLAGS: {:#018x} CS: {:?} SS: {:?}\n",
        //     self.int_frame.cpu_flags,
        //     self.int_frame.code_segment,
        //     self.int_frame.stack_segment
        // )?;

        Ok(())
    }
}
