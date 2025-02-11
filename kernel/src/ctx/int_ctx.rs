use core::{arch::naked_asm, fmt::Display, mem::offset_of};

use x86_64::structures::idt::{InterruptStackFrame, InterruptStackFrameValue, PageFaultErrorCode};

use crate::ctx::ctx_test;

#[repr(C)]
#[derive(Debug)]
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
pub struct InterruptContext {
    pub context: Context,
    pub int_frame: InterruptStackFrame,
}

impl InterruptContext {
    pub const fn zero() -> InterruptContext {
        unsafe { core::mem::zeroed() }
    }
    // TODO: The const here is a bit of a lie due to InterruptStackFrame not having a const constructor.
    pub const fn zero_with_frame(frame: InterruptStackFrame) -> InterruptContext {
        let mut ctx = Self::zero();
        ctx.int_frame = frame;
        ctx
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PageFaultInterruptContext {
    pub context: Context,
    pub int_frame: InterruptStackFrame,
    pub error_code: PageFaultErrorCode,
}

impl PageFaultInterruptContext {
    pub const fn zero() -> PageFaultInterruptContext {
        unsafe { core::mem::zeroed() }
    }
}

/// Wrapper for an interrupt handler. Based on [this implementation](https://github.com/bendudson/EuraliOS/blob/main/kernel/src/interrupts.rs#L117)
/// which is further based on another rust os which I'm not bothering to go down the rabbit hole to find.
#[macro_export]
macro_rules! interrupt_wrapper {
    ($handler: ident, $raw: ident) => {
        #[naked]
        pub extern "x86-interrupt" fn $raw(_: InterruptStackFrame) {
            unsafe {
                ::core::arch::naked_asm! {
                    // Disable interrupts.
                    "cli",
                    // Push all registers to the stack. Push the registers in the OPPOSITE order that they are defined in InterruptRegisters.
                    "push rax",
                    "push rbx",
                    "push rcx",
                    "push rdx",
                    "push rbp",
                    "push rdi",
                    "push rsi",
                    "push r8",
                    "push r9",
                    "push r10",
                    "push r11",
                    "push r12",
                    "push r13",
                    "push r14",
                    "push r15",

                    // TODO: We don't do any floating point stuff yet, so we don't need to save the floating point registers.

                    // C abi requires that the first parameter is in rdi, so we need to move the stack pointer to rdi.
                    "mov rdi, rsp",
                    "call {handler}",

                    // Pop all registers from the stack. Pop the registers in the SAME order that they are defined in InterruptRegisters.
                    "pop r15",
                    "pop r14",
                    "pop r13",
                    "pop r12",
                    "pop r11",
                    "pop r10",
                    "pop r9",
                    "pop r8",
                    "pop rsi",
                    "pop rdi",
                    "pop rbp",
                    "pop rdx",
                    "pop rcx",
                    "pop rbx",
                    "pop rax",

                    // Re-enable interrupts.
                    "sti",
                    // Return from interrupt.
                    "iretq",
                    handler = sym $handler,
                }
            }
        }
    };
}

interrupt_wrapper!(ctx_test, ctx_test_raw);
