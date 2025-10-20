use x86_64::registers::control::Cr2;

use crate::{
    context::{InterruptCodeContext, InterruptContext, PageFaultInterruptContext},
    interrupt_wrapper,
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
    loop {
        x86_64::instructions::hlt();
    }
}

interrupt_wrapper!(panic_handler, panic_handler_raw);
