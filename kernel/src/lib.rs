#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::arch::asm;

use limine::request::StackSizeRequest;
use log::info;
pub(crate) use spin::Once;

mod gdt;
pub mod interrupts;
pub mod panic;
pub mod serial;
mod util;

/// Because we need a relatively big stack for the display, we need to request a bigger stack size
/// from the bootloader.
// TODO: For some ungodly reason, increasing this causes something to go wrong with the framebuffer??.
// I have some probably incorrect theories about why this is happening.
// My guess is that the large stack size goes into the framebuffer info struct, which then makes it corrupted.
// I have not done enough research to confirm this, but it's my best guess.
const STACK_SIZE: u64 = 0xFF000; // 0xCF8
#[used]
static STACK_REQUEST: StackSizeRequest = StackSizeRequest::new().with_size(STACK_SIZE);

// sb: 0xffff80003f74beb0,sp:0xffff80003f74c770
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
    info!("Initialized serial");
    gdt::init_gdt();
    info!("Initialized GDT");
    interrupts::init();
    info!("Initialized interrupts");
    info!("Kernel initialized");
}
