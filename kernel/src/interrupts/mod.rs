use core::{convert::Infallible, mem};

use spin::{Mutex, MutexGuard, Once};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

mod exception;
pub mod hardware;
mod macros;

use crate::{
    context::{InterruptCodeContext, InterruptContext, PageFaultInterruptContext},
    declare_module, init_interrupt_table, interrupt_wrapper,
};

pub static IDT: InterruptTable = InterruptTable::uninitialized();

/// The interrupt table for the kernel.
pub struct InterruptTable {
    table: Mutex<InterruptDescriptorTable>,
    exchange: Mutex<InterruptDescriptorTable>,
    init: Once<()>,
}

pub type CodeHandler = fn(ctx: *mut InterruptCodeContext, index: u8, name: &'static str);
pub type InterruptHandler = fn(ctx: *mut InterruptContext, index: u8, name: &'static str);
pub type PageFaultHandler = fn(*mut PageFaultInterruptContext);

impl InterruptTable {
    /// Create a new, uninitialized interrupt table.
    pub const fn uninitialized() -> InterruptTable {
        InterruptTable {
            table: Mutex::new(InterruptDescriptorTable::new()),
            exchange: Mutex::new(InterruptDescriptorTable::new()),
            init: Once::new(),
        }
    }
    /// Commit the table, returning the old table if it was initialized.
    ///
    /// # Safety
    /// The caller must ensure that all modifications to the exchange table are complete and will not violate memory safety.
    ///
    /// The caller must ensure that interrupts are disabled when calling this function.
    pub unsafe fn commit(&'static self) -> Option<InterruptDescriptorTable> {
        let mut old = None;
        let mut table = self.table.try_lock().expect("Interrupt table is locked");

        // If the table is uninitialized, initialize it.
        if !self.init.is_completed() {
            // Interrupts are disabled, so we can safely initialize the table.
            self.init.call_once(|| ());
            // Get the raw pointer to the table, convert it to &'static, and load it.
            unsafe {
                mem::transmute::<&InterruptDescriptorTable, &'static InterruptDescriptorTable>(
                    &*table as &InterruptDescriptorTable,
                )
                .load();
            }
        } else {
            old = Some(mem::replace(&mut *table, InterruptDescriptorTable::new()));
        }
        // Copy the table to the exchange table.
        let exchange = self
            .exchange
            .try_lock()
            .expect("Interrupt exchange table is locked");

        // Copy the exchange table to the real table.
        table.clone_from(&*exchange);

        old
    }

    /// Return a guard to the interrupt table.
    /// Modifications to this table *will not* take effect until `commit` is called,
    /// and this table may not be a complete representation of the loaded interrupt table.
    pub fn modify(&self) -> MutexGuard<InterruptDescriptorTable> {
        self.exchange
            .try_lock()
            .expect("Interrupt exchange table is locked")
    }
}

declare_module!("interrupts", init);

pub fn init() -> Result<(), Infallible> {
    x86_64::instructions::interrupts::disable();
    init_interrupt_table!(
        exception::general_code_handler,
        exception::page_fault_handler,
        exception::general_handler,
        IDT
    );
    unsafe {
        IDT.commit();
    }

    hardware::define_hardware();
    x86_64::instructions::interrupts::enable();
    Ok(())
}

const BASIC_HANDLERS: [&'static str; 32] = [
    "DIVIDE ERROR",
    "DEBUG",
    "NON MASKABLE INTERRUPT",
    "BREAKPOINT",
    "OVERFLOW",
    "BOUND RANGE EXCEEDED",
    "INVALID OPCODE",
    "DEVICE NOT AVAILABLE",
    "DOUBLE FAULT",
    "COPROCESSOR SEGMENT OVERRUN",
    "INVALID TSS",
    "SEGMENT NOT PRESENT",
    "STACK SEGMENT FAULT",
    "GENERAL PROTECTION FAULT",
    "PAGE FAULT",
    "RESERVED",
    "X87 FLOATING POINT EXCEPTION",
    "ALIGNMENT CHECK",
    "MACHINE CHECK",
    "SIMD FLOATING POINT EXCEPTION",
    "VIRTUALIZATION EXCEPTION",
    "CONTROL PROTECTION EXCEPTION",
    "RESERVED",
    "RESERVED",
    "RESERVED",
    "RESERVED",
    "RESERVED",
    "RESERVED",
    "HYPERVISOR INJECTION EXCEPTION",
    "VMM COMMUNICATION EXCEPTION",
    "SECURITY EXCEPTION",
    "RESERVED",
];
