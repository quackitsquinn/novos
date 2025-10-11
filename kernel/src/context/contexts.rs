//! General context structs for interrupts/context switching.
use core::fmt::Display;

use x86_64::structures::idt::{InterruptStackFrameValue, PageFaultErrorCode};

/// Represents the CPU context during an interrupt.
/// Contains the values of all general-purpose registers.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct ContextValue {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    // TODO float regs
}

impl ContextValue {
    /// Returns a zeroed context value.
    pub const fn zero() -> ContextValue {
        unsafe { core::mem::zeroed() }
    }
}

impl Display for ContextValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("\n")?;
        macro_rules! p {
            ($($arg:tt)*) => {
                f.write_fmt(format_args!($($arg)*))
            };
        }
        p!(
            "R15: {:#018x}, R14: {:#018x}, R13: {:#018x}\n",
            self.r15,
            self.r14,
            self.r13
        )?;

        p!(
            "R12: {:#018x}, R11: {:#018x}, R10: {:#018x}\n",
            self.r12,
            self.r11,
            self.r10
        )?;

        p!(
            "R09: {:#018x}, R08: {:#018x}, RSI: {:#018x}\n",
            self.r9,
            self.r8,
            self.rsi
        )?;

        p!(
            "RDI: {:#018x}, RBP: {:#018x}, RDX: {:#018x}\n",
            self.rdi,
            self.rbp,
            self.rdx
        )?;

        p!(
            "RCX: {:#018x}, RBX: {:#018x}, RAX: {:#018x}\n",
            self.rcx,
            self.rbx,
            self.rax
        )?;

        Ok(())
    }
}

/// Represents the CPU context during a page fault interrupt.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PageFaultInterruptContextValue {
    /// The general CPU context at the time of the page fault.
    pub context: ContextValue,
    /// The interrupt stack frame at the time of the page fault.
    pub int_frame: InterruptStackFrameValue,
    /// The page fault error code associated with the page fault.
    pub error_code: PageFaultErrorCode,
}

impl PageFaultInterruptContextValue {
    /// Returns a zeroed page fault interrupt context value.
    pub const fn zero() -> PageFaultInterruptContextValue {
        unsafe { core::mem::zeroed() }
    }
}
