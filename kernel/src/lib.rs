#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::arch::asm;

use limine::request::StackSizeRequest;
pub(crate) use spin::{Mutex, Once};

pub(crate) type OnceMut<T> = Once<Mutex<T>>;

pub mod display;
mod gdt;
pub mod interrupts;
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

static DISPLAY_INITIALIZED: Once<()> = Once::new();

/// Returns true if the display has been initialized. Intended for use in stuff like panic functions, which can occur before the display is initialized.
pub fn display_init() -> bool {
    if DISPLAY_INITIALIZED.is_completed() {
        return true;
    }
    return false;
}

pub fn init_kernel() {
    serial::init();
    println!("Initialized serial");
    gdt::init_gdt();
    println!("Initialized GDT");
    interrupts::init();
    println!("Initialized interrupts");
    sprintln!("Checking if bootloader has provided stack size");
    // If the response is present, the bootloader has provided our requested stack size.
    if let Some(_) = STACK_REQUEST.get_response() {
        sprintln!("Bootloader has provided stack size: 0x{:x}", STACK_SIZE);
    } else {
        sprintln!("Bootloader has not provided stack size");
    }
    lazy_static::initialize(&display::FRAMEBUFFER);
    sprintln!("Initialized framebuffer");
    lazy_static::initialize(&display::TERMINAL);
    sprintln!("Initialized terminal");
    DISPLAY_INITIALIZED.call_once(|| ());
}
