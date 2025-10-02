#![no_std]
#![no_main]
/* FEATURES */
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![feature(allocator_api)]
#![feature(pointer_is_aligned_to)]
#![feature(optimize_attribute)]
/* LINT OPTS */
#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_debug_implementations)]
/* TEST RUNNER */
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::arch::asm;

use cake::declare_module;

use cake::limine::BaseRevision;
use cake::limine::request::StackSizeRequest;
use interrupts::hardware;
use kserial::client::get_serial_client;
use log::info;
use spin::Once;

use crate::memory::stack::Stack;
use crate::mp::mp_setup;

pub mod acpi;
pub mod context;
pub mod display;
mod gdt;
mod interpreter;
pub mod interrupts;
pub mod memory;
pub mod mp;
pub mod panic;
pub mod pci;
pub mod proc;
mod requests;
pub mod serial;
pub mod testing;

pub const STACK_SIZE: u64 = 1 << 18;
static STACK_SIZE_REQUEST: StackSizeRequest = StackSizeRequest::new().with_size(STACK_SIZE);

pub static STACK_BASE: Once<u64> = Once::new();

#[used]
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
#[unsafe(no_mangle)]
pub extern "sysv64" fn init_kernel(rsp: u64) -> ! {
    STACK_BASE.call_once(|| rsp);
    x86_64::instructions::interrupts::disable();
    unsafe {
        init_kernel_services();
    }
    info!("Kernel initialized! Entering hlt loop");
    x86_64::instructions::interrupts::disable();
    interpreter::run();
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

    if !STACK_SIZE_REQUEST.get_response().is_some() {
        panic!("Failed to get stack size from Limine");
    }

    gdt::MODULE.init();
    interrupts::MODULE.init();
    hardware::MODULE.init();

    if option_env!("NO_SERIAL").is_none() {
        get_serial_client().enable_packet_support();
    }

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
