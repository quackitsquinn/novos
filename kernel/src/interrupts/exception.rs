use crate::{
    context::{InterruptCodeContext, InterruptContext, PageFaultInterruptContext},
    panic::stacktrace::{self, StackFrame},
    println,
};

pub fn general_handler(ctx: *mut InterruptContext, _: u8, name: &'static str) {
    let ctx = unsafe { &mut *ctx };
    println!("===== {} =====", name);
    println!("(no error code)");
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    loop {}
}

pub fn general_code_handler(ctx: *mut InterruptCodeContext, _: u8, name: &'static str) {
    let ctx = unsafe { &mut *ctx };
    println!("===== {} =====", name);
    println!("ERROR CODE: {:?}", ctx.code);
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    loop {}
}

pub fn page_fault_handler(page_fault_ctx: *mut PageFaultInterruptContext) {
    let ctx = unsafe { &mut *page_fault_ctx };
    println!("===== PAGE FAULT =====");
    println!("{:?}", ctx.error_code);
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    loop {}
}
