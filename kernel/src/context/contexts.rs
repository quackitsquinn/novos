use core::fmt::Display;

use x86_64::structures::idt::{InterruptStackFrameValue, PageFaultErrorCode};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Context {
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
}

impl Context {
    pub const fn zero() -> Context {
        unsafe { core::mem::zeroed() }
    }
}

impl Display for Context {
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

#[repr(C)]
#[derive(Debug)]
pub struct PageFaultInterruptContext {
    pub context: Context,
    pub int_frame: InterruptStackFrameValue,
    pub error_code: PageFaultErrorCode,
}

impl PageFaultInterruptContext {
    pub const fn zero() -> PageFaultInterruptContext {
        unsafe { core::mem::zeroed() }
    }
}
