#![no_std]
#![no_main]

use core::arch::asm;

use limine::request::StackSizeRequest;
pub(crate) use spin::{Mutex, Once};

pub(crate) type OnceMut<T> = Once<Mutex<T>>;

pub mod display;
mod gdt;
pub mod serial;

/// Because we need a relatively big stack for the display, we need to request a bigger stack size
/// from the bootloader.
const STACK_SIZE: u64 = 0x32000; // 0xCF8
#[used]
static STACK_REQUEST: StackSizeRequest = StackSizeRequest::new().with_size(STACK_SIZE);

/// Halts the CPU indefinitely.
pub fn hlt_loop() -> ! {
    // SAFETY: We only call cli and hlt, which are safe to call.
    unsafe { asm!("cli") };
    loop {
        unsafe { asm!("hlt") };
    }
}

pub fn init_kernel() {
    serial::init();
    gdt::init_gdt();
    sprintln!("Initialized serial");
    sprintln!("Checking if bootloader has provided stack size");
    // If the response is present, the bootloader has provided our requested stack size.
    if let Some(stack_size) = STACK_REQUEST.get_response() {
        sprintln!("Bootloader has provided stack size: 0x{:x}", STACK_SIZE);
    } else {
        sprintln!("Bootloader has not provided stack size");
    }
    lazy_static::initialize(&display::FRAMEBUFFER);
    sprintln!("Initialized framebuffer");
    lazy_static::initialize(&display::TERMINAL);
    sprintln!("Initialized terminal");
}
