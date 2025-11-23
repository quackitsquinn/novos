use core::sync::atomic::{AtomicU64, Ordering};

use cake::log::info;
use x86_64::registers::control::Cr2;

use crate::{
    context::{InterruptCodeContext, InterruptContext, PageFaultInterruptContext},
    interrupt_wrapper, interrupts,
    mp::{self, LAPIC},
    panic::stacktrace::{self, StackFrame},
    println,
};

#[inline(never)]
#[unsafe(no_mangle)]
extern "C" fn exception_brk() {}

pub fn general_handler(ctx: InterruptContext, _: u8, name: &'static str) {
    println!("===== {} =====", name);
    println!("(no error code)");
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    unsafe { stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame) };
    exception_brk();
    loop {}
}

pub fn general_code_handler(ctx: InterruptCodeContext, _: u8, name: &'static str) {
    println!("===== {} =====", name);
    println!("ERROR CODE: {:?}", ctx.code);
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    unsafe { stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame) };
    exception_brk();
    loop {}
}

pub fn page_fault_handler(ctx: PageFaultInterruptContext) {
    println!("===== PAGE FAULT =====");
    println!("{:?}: {:?}", ctx.error_code, Cr2::read());
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    unsafe { stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame) };
    exception_brk();
    loop {}
}

pub extern "C" fn panic_handler(_: InterruptContext) {
    interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}

interrupt_wrapper!(panic_handler, panic_handler_raw);

pub extern "C" fn spurious_handler(_: InterruptContext, _: u8) {
    // Currently we don't really care about spurious interrupts, but logging them might help in debugging.
    info!(
        "Spurious interrupt received on core {}",
        mp::current_core_id()
    );

    unsafe {
        LAPIC.eoi();
    }
}

interrupt_wrapper!(spurious_handler, spurious_handler_raw);

pub extern "C" fn apic_error(_: InterruptContext, _: u8) {
    info!(
        "APIC error interrupt received on core {}: {}",
        mp::current_core_id(),
        unsafe { LAPIC.read_offset::<u32>(0x280) }
    );

    unsafe {
        LAPIC.eoi();
    }
}

interrupt_wrapper!(apic_error, apic_error_raw);

pub extern "C" fn timer_handler(_: InterruptContext, _: u8) {}

interrupt_wrapper!(timer_handler, timer_handler_raw);
