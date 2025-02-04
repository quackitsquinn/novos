use core::{arch::naked_asm, mem::offset_of};

use x86_64::structures::idt::{InterruptStackFrame, InterruptStackFrameValue};

use crate::ctx::ctx_test;

#[repr(C)]
#[derive(Debug)]
pub struct InterruptContext {
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
    pub int_frame: InterruptStackFrame,
}

impl InterruptContext {
    pub fn empty() -> InterruptContext {
        InterruptContext {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            int_frame: unsafe { core::mem::zeroed() },
        }
    }
}

/// Wrapper for an interrupt handler. Based on [this implementation](https://github.com/bendudson/EuraliOS/blob/main/kernel/src/interrupts.rs#L117)
/// which is further based on another rust os which I'm not bothering to go down the rabbit hole to find.
macro_rules! interrupt_wrapper {
    ($handler: ident, $raw: ident) => {
        #[naked]
        pub extern "x86-interrupt" fn $raw(_: InterruptStackFrame) {
            unsafe {
                naked_asm! {
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
