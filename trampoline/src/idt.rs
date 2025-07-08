use log::info;
use spin::Once;
use x86_64::{
    set_general_handler,
    structures::idt::{GeneralHandlerFunc, InterruptDescriptorTable, InterruptStackFrame},
};

static IDT: Once<InterruptDescriptorTable> = Once::new();

pub fn load() {
    let mut idt = InterruptDescriptorTable::new();
    set_general_handler!(&mut idt, general_handler);
    IDT.call_once(|| idt);
    IDT.get().unwrap().load();
}

fn general_handler(frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    panic!(
        "General handler called for interrupt {} with error code {:?}\n{:#?}",
        index, error_code, frame
    );
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
