#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![forbid(unsafe_op_in_unsafe_fn)]
#![feature(custom_test_frameworks)]
#![feature(maybe_uninit_uninit_array)]
#![feature(allocator_api)]
#![feature(pointer_is_aligned_to)]
#![feature(naked_functions)]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::arch::asm;

use interrupts::{set_custom_handler, CUSTOM_HANDLERS};
use limine::BaseRevision;
use log::info;
use spin::Once;

pub mod ctx;
pub mod display;
mod gdt;
pub mod interrupts;
pub mod memory;
pub mod panic;
pub mod pci;
pub mod serial;
pub mod testing;
pub mod util;

const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const STACK_SIZE: u64 = 1 << 16; // Limine defaults to 16KiB

static BASE_REVISION: BaseRevision = BaseRevision::with_revision(3);

/// Halts the CPU indefinitely.
pub fn hlt_loop() -> ! {
    // SAFETY: We only call cli and hlt, which are safe to call.
    unsafe { asm!("cli") };
    loop {
        unsafe { asm!("hlt") };
    }
}

/// Initializes the kernel and takes over the system.
/// This function should be called from the `_start` function.
pub fn init_kernel() -> ! {
    unsafe {
        init_kernel_services();
    }
    // TODO: init_kernel_runtime(); or something similar
    hlt_loop()
}

/// Loads all the kernel services that will not take over the system.
///
/// # Safety
/// The caller *must* ensure that this function is only called once. Calling it more than once will
/// result in undefined behavior.
pub(crate) unsafe fn init_kernel_services() {
    static INIT: Once<()> = Once::new();
    if INIT.is_completed() {
        panic!("init_kernel_services called more than once");
    }
    INIT.call_once(|| ());
    serial::MODULE.init();
    panic::MODULE.init();
    gdt::MODULE.init();
    interrupts::MODULE.init();
    memory::MODULE.init();
    #[cfg(not(test))] // Tests don't have a display
    display::MODULE.init();
    pci::MODULE.init();
    info!("Kernel services initialized");
}

#[macro_export]
macro_rules! debug_release_select {
    (debug $run_in_debug: block, release $run_in_release: block ) => {{
        #[cfg(debug_assertions)]
        $run_in_debug
        #[cfg(not(debug_assertions))]
        $run_in_release
    }};
}
