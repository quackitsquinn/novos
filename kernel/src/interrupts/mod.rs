use core::{convert::Infallible, error, mem};

use log::{error, info};
use spin::{Mutex, Once};
use x86_64::{
    set_general_handler,
    structures::idt::{
        HandlerFunc, InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
    },
};

pub mod hardware;

use crate::{
    ctx::PageFaultInterruptContext,
    declare_module, interrupt_wrapper,
    panic::stacktrace::{self, StackFrame},
};

static IDT: Once<InterruptDescriptorTable> = Once::new();
// no clue if i will use these (or even how) but they are here
pub(crate) static CUSTOM_HANDLERS: Mutex<[Option<HandlerFunc>; 256 - 32]> =
    Mutex::new([None; 256 - 32]);

pub fn set_custom_handler(index: u8, handler: HandlerFunc) {
    if index < 32 {
        panic!("Cannot set a custom handler for a basic interrupt");
    }
    let mut handlers = CUSTOM_HANDLERS.lock();
    handlers[index as usize - 32] = Some(handler);
}

pub static UNHANDLED_INTERRUPT: Once<()> = Once::new();
// General handler
fn general_handler(stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    UNHANDLED_INTERRUPT.call_once(|| ());
    error!("Interrupt: {} ({})", index, BASIC_HANDLERS[index as usize]);
    error!("Error code: {:?}", error_code);
    error!("{:?}", stack_frame);
    panic!("Unhandled interrupt");
}

extern "C" fn page_fault_handler(page_fault_ctx: *mut PageFaultInterruptContext) -> ! {
    UNHANDLED_INTERRUPT.call_once(|| ());
    let ctx = unsafe { &mut *page_fault_ctx };
    error!("Page fault: {}", ctx.context);
    error!("Error code: {:?}", ctx.error_code);
    error!("STACK TRACE:");
    stacktrace::print_trace_raw(ctx.context.rbp as *const StackFrame);
    loop {}
}

interrupt_wrapper!(page_fault_handler, raw_page_fault_handler);

declare_module!("interrupts", init);

fn init() -> Result<(), Infallible> {
    // Initialize hardware interrupts
    info!("Defining hardware interrupts");
    hardware::define_hardware();
    IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        set_general_handler!(&mut idt, general_handler);
        idt.page_fault.set_handler_fn(unsafe {
            mem::transmute(raw_page_fault_handler as extern "x86-interrupt" fn(InterruptStackFrame))
        });
        for (i, handler) in CUSTOM_HANDLERS
            .lock()
            .iter()
            .enumerate()
            .filter(|(_, h)| h.is_some())
            .map(|(i, h)| (i, h.unwrap()))
        {
            idt[i as u8 + 32].set_handler_fn(handler);
        }
        // println!("{:?}", idt.breakpoint);
        //       println!("{:?}", idt);
        idt
    });
    // Load the IDT now that it is & 'static
    IDT.get().unwrap().load();
    hardware::MODULE.init();
    Ok(())
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
