//! Rust abstractions for handling interrupts and IRQs.
use core::{convert::Infallible, mem};

use cake::{Mutex, MutexGuard, Once};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

mod exception;
pub mod hardware;

pub mod local;
mod lock;
mod macros;

use crate::{
    context::{InterruptCodeContext, InterruptContext, PageFaultInterruptContext},
    declare_module, init_idt, interrupt_wrapper,
    interrupts::local::LocalIdt,
};

pub use lock::{InterruptMutex, InterruptMutexGuard};

/// The local IDT for each core.
pub static IDT: LocalIdt = LocalIdt::new();

/// A handler for interrupts with an error code.
pub type CodeHandler = fn(ctx: InterruptCodeContext, index: u8, name: &'static str);
/// A basic interrupt handler.
pub type InterruptHandler = fn(ctx: InterruptContext, index: u8, name: &'static str);
/// A handler for page fault interrupts.
pub type PageFaultHandler = fn(ctx: PageFaultInterruptContext);

declare_module!("interrupts", init);

fn init() -> Result<(), Infallible> {
    x86_64::instructions::interrupts::disable();

    let mut idt = IDT.get_mut();

    init_idt!(
        exception::general_code_handler,
        exception::page_fault_handler,
        exception::general_handler,
        &mut idt
    );

    drop(idt);
    hardware::define_hardware();
    IDT.swap_and_sync();
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

/// Disables interrupts.
pub fn disable() {
    x86_64::instructions::interrupts::disable();
}

/// Enables Interrupts
pub fn enable() {
    x86_64::instructions::interrupts::enable();
}

/// Returns `true` if interrupts are enabled.
pub fn are_enabled() -> bool {
    x86_64::instructions::interrupts::are_enabled()
}

/// Executes a closure without interrupts.
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    x86_64::instructions::interrupts::without_interrupts(f)
}
