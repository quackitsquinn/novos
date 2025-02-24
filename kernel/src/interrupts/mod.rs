use core::{convert::Infallible, error, mem};

use log::{error, info};
use spin::{Mutex, MutexGuard, Once};
use x86_64::{
    set_general_handler,
    structures::idt::{
        Entry, HandlerFunc, InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
    },
};

mod exception;
pub mod hardware;

use crate::{
    ctx::{PageFaultInterruptContext, ProcessorContext},
    declare_module, interrupt_wrapper,
    panic::stacktrace::{self, StackFrame},
    println,
    util::{InterruptBlock, OnceMutex},
};

pub static IDT: InterruptTable = InterruptTable::uninitialized();

/// The interrupt table for the kernel.
pub struct InterruptTable {
    table: OnceMutex<InterruptDescriptorTable>,
    exchange: OnceMutex<InterruptDescriptorTable>,
}

impl InterruptTable {
    /// Create a new, uninitialized interrupt table.
    pub const fn uninitialized() -> InterruptTable {
        InterruptTable {
            table: OnceMutex::uninitialized(),
            exchange: OnceMutex::uninitialized(),
        }
    }
    /// Commit the table, returning the old table if it was initialized.
    ///
    /// # Safety
    /// The caller must ensure that all modifications to the exchange table are complete and will not violate memory safety.
    pub unsafe fn commit(&self) -> Option<InterruptDescriptorTable> {
        let mut old = None;
        x86_64::instructions::interrupts::without_interrupts(|| {
            // If the table is uninitialized, initialize it.
            if !self.table.is_initialized() {
                // Interrupts are disabled, so we can safely initialize the table.
                self.table.init(InterruptDescriptorTable::new());
                // Get the raw pointer to the table, convert it to &'static, and load it.
                let table = self.table.get();
                unsafe {
                    (&*((&*table) as *const InterruptDescriptorTable)).load();
                }
                drop(table);
            } else {
                let mut table = self.table.get();
                old = Some(mem::replace(&mut *table, InterruptDescriptorTable::new()));
            }
            // Copy the table to the exchange table.
            let mut table = self.table.get();
            let exchange = self.exchange.get();

            // Copy the exchange table to the real table.
            table.clone_from(&*exchange);
        });
        old
    }

    /// Return a guard to the interrupt table.
    /// Modifications to this table *will not* take effect until `commit` is called,
    /// and this table may not be a complete representation of the loaded interrupt table.
    pub fn modify(&self) -> MutexGuard<InterruptDescriptorTable> {
        self.exchange.get()
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
