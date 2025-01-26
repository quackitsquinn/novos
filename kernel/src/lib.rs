#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![forbid(unsafe_op_in_unsafe_fn)]
#![feature(custom_test_frameworks)]
#![feature(maybe_uninit_uninit_array)]
#![feature(allocator_api)]
#![feature(pointer_is_aligned_to)]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::arch::asm;

use limine::BaseRevision;
use log::info;

pub mod acpi;
pub mod display;
mod gdt;
pub mod interrupts;
pub mod memory;
pub mod panic;
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

pub fn init_kernel() {
    serial::MODULE.init();
    panic::MODULE.init();
    gdt::MODULE.init();
    interrupts::MODULE.init();
    memory::MODULE.init();

    #[cfg(not(test))] // Tests don't have a display
    display::MODULE.init();

    info!("Kernel initialized");
}

#[macro_export]
macro_rules! debug_release_select {
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
