use log::error;
use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    ctx::PageFaultInterruptContext,
    panic::stacktrace::{self, StackFrame},
    println,
};

fn general_handler(stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    error!("Interrupt: {} ({})", index, BASIC_HANDLERS[index as usize]);
    error!("Error code: {:?}", error_code);
    error!("{:?}", stack_frame);
    panic!("Unhandled interrupt");
}

extern "C" fn page_fault_handler(page_fault_ctx: *mut PageFaultInterruptContext) -> ! {
    let ctx = unsafe { &mut *page_fault_ctx };
    println!("===== PAGE FAULT =====");
    println!("{:?}", ctx.error_code);
    println!("== CPU STATE ==");
    println!("{}", ctx.context);
    println!("== STACK TRACE ==");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    loop {}
}

const BASIC_HANDLERS: [&'static str; 32] = [
    "Divide Error",
    "Debug",
    "Non Maskable Interrupt",
    "Breakpoint",
    "Overflow",
    "Bound Range Exceeded",
    "Invalid Opcode",
    "Device Not Available",
    "Double Fault",
    "Coprocessor Segment Overrun",
    "Invalid TSS",
    "Segment Not Present",
    "Stack Segment Fault",
    "General Protection Fault",
    "Page Fault",
    "Reserved",
    "x87 Floating Point Exception",
    "Alignment Check",
    "Machine Check",
    "SIMD Floating Point Exception",
    "Virtualization Exception",
    "Control Protection Exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Hypervisor Injection Exception",
    "VMM Communication Exception",
    "Security Exception",
    "Reserved",
];
