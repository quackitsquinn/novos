//! The core kernel module post bootloader handoff.
//! Contains scheduling and process management.
#![no_std]
#![no_main]
/* FEATURES */
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![feature(allocator_api)]
#![feature(pointer_is_aligned_to)]
/* LINT OPTS */
#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
/* TEST RUNNER */
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::arch::asm;

use cake::declare_module;

use cake::limine::BaseRevision;
use cake::log::info;
use cake::spin::Once;
use interrupts::hardware;
use kserial::client::get_serial_client;

use crate::mp::mp_setup;

pub mod acpi;
pub mod context;
pub mod display;
mod gdt;
pub mod interrupts;
pub mod memory;
pub mod mp;
pub mod panic;
pub mod pci;
pub mod proc;
pub mod requests;
pub mod serial;
pub mod testing;

/// The size of the kernel stack in bytes.
pub const STACK_SIZE: u64 = 1 << 16; // Limine defaults to 16KiB

/// The base address of the kernel stack. Set by the function that calls [init_kernel].
pub static STACK_BASE: Once<u64> = Once::new();

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
#[unsafe(no_mangle)]
pub extern "sysv64" fn init_kernel(rsp: u64) -> ! {
    STACK_BASE.call_once(|| rsp);
    x86_64::instructions::interrupts::disable();
    unsafe {
        init_kernel_services();
    }
    info!("Kernel initialized! Entering hlt loop");
    x86_64::instructions::interrupts::disable();
    loop {}
}

/// Loads all the kernel services that will not take over the system.
///
/// # Safety
/// The caller *must* ensure that this function is only called once. Calling it more than once will
/// result in undefined behavior.
///
/// The caller must also ensure that [STACK_BASE] has been initialized.
pub(crate) unsafe fn init_kernel_services() {
    // Ensure this function is only called once.
    // This is subject to removal so don't bank on it, hence why it's unsafe.
    static INIT: Once<()> = Once::new();
    if INIT.is_completed() {
        panic!("init_kernel_services called more than once");
    }
    INIT.call_once(|| ());
    serial::MODULE.init();
    requests::MODULE.init();
    panic::MODULE.init();
    gdt::MODULE.init();
    interrupts::MODULE.init();
    hardware::MODULE.init();
    get_serial_client().enable_packet_support();
    // {
    //     let e = File::create_file("test.txt").unwrap();
    //     e.write(b"Hello, world!").unwrap();
    //     unsafe {
    //         e.close();
    //     }
    // }

    // test_two_way_serial();
    memory::MODULE.init();
    mp_setup::MODULE.init();
    memory::paging::kernel::MODULE.init();
    #[cfg(not(test))] // Tests don't have a display
    display::MODULE.init();
    acpi::MODULE.init();
    mp::MODULE.init();
    pci::MODULE.init();
    proc::MODULE.init();
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
