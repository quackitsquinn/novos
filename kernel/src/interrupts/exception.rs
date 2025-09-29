use core::mem;

use x86_64::{registers::control::Cr2, structures::gdt::SegmentSelector};

use crate::{
    context::{InterruptCodeContext, InterruptContext, PageFaultInterruptContext},
    panic::stacktrace::{self, StackFrame},
    print, println,
};

#[inline(never)]
#[no_mangle]
extern "C" fn exception_brk() {}

pub fn general_handler(ctx: InterruptContext, _: u8, name: &'static str) {
    println!("===== {} =====", name);
    println!("(no error code)");
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    exception_brk();
    loop {}
}

const GENERAL_PROTECTION_FAULT_ID: u8 = 13;

pub fn general_code_handler(ctx: InterruptCodeContext, id: u8, name: &'static str) {
    println!("===== {} =====", name);
    match id {
        GENERAL_PROTECTION_FAULT_ID => {
            print!("General Protection Fault!");
            if ctx.code != 0 {
                let selector: SegmentSelector = unsafe { mem::transmute(ctx.code as u16) };
                println!(" Error Code: {:?}", selector);
            }
        }
        _ => {
            println!("Error Code: {:?}", ctx.code);
        }
    }

    println!("== CPU STATE ==");
    println!("{}", ctx);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    exception_brk();
    loop {}
}

pub fn page_fault_handler(ctx: PageFaultInterruptContext) {
    println!("===== PAGE FAULT =====");
    println!("{:?}: {:?}", ctx.error_code, Cr2::read());
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    exception_brk();
    loop {}
}
