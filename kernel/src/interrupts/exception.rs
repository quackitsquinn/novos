use x86_64::registers::control::{Cr2, Cr3};

use crate::{
    context::{
        InterruptCodeContext, InterruptCodeContextValue, InterruptContext, InterruptContextValue,
        PageFaultInterruptContext, PageFaultInterruptContextValue,
    },
    panic::stacktrace::{self, StackFrame},
    println,
};

pub fn general_handler(ctx: InterruptContext, _: u8, name: &'static str) {
    println!("===== {} =====", name);
    println!("(no error code)");
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    loop {}
}

pub fn general_code_handler(ctx: InterruptCodeContext, _: u8, name: &'static str) {
    println!("===== {} =====", name);
    println!("ERROR CODE: {:?}", ctx.code);
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    loop {}
}

pub fn page_fault_handler(ctx: PageFaultInterruptContext) {
    println!("===== PAGE FAULT =====");
    println!("{:?}: {:?}", ctx.error_code, Cr2::read());
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    loop {}
}
