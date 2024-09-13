#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

use core::arch::asm;

use limine::request::StackSizeRequest;
use log::info;
pub(crate) use spin::{Mutex, Once};

pub mod display;
mod gdt;
pub mod interrupts;
pub mod memory;
pub mod serial;
mod util;

/// Because we need a relatively big stack for the display, we need to request a bigger stack size
/// from the bootloader.
const STACK_SIZE: u64 = 0x64F00; // 0xCF8
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
    info!("Initialized serial");
    gdt::init_gdt();
    info!("Initialized GDT");
    interrupts::init();
    info!("Initialized interrupts");
    serial::init_debug_harness();
    info!("Initialized debug harness");
    memory::init();
    info!("Initialized paging");
    info!("Checking if bootloader has provided stack size");
    // If the response is present, the bootloader has provided our requested stack size.
    if let Some(_) = STACK_REQUEST.get_response() {
        info!("Bootloader has provided stack size: 0x{:x}", STACK_SIZE);
    } else {
        info!("Bootloader has not provided stack size");
    }
    info!("Initializing display");
    display::init();
    DISPLAY_INITIALIZED.call_once(|| ());
    info!("Kernel initialized");

    let _ = debug_release_check!(
        debug {
            sprintln!("Debug build");
            3
        },
        release {
            sprintln!("Release build");
            33
        }
    );
}

#[macro_export]
macro_rules! debug_release_check {
    (debug $run_in_debug: tt, release $run_in_release: tt ) => {{
        #[cfg(debug_assertions)]
        $run_in_debug
        #[cfg(not(debug_assertions))]
        $run_in_release
    }};
}

#[macro_export]
macro_rules! assert_or_else {
    ($assertion: expr, $else_block: block) => {
        if !$assertion {
            $else_block
        }
    };
}
