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

use alloc::boxed::Box;
use interrupts::hardware;
use limine::BaseRevision;
use log::info;
use proc::{sched, KERNEL_THREAD_SCHEDULER};
use spin::Once;

pub mod context;
pub mod display;
mod gdt;
pub mod interrupts;
pub mod memory;
pub mod panic;
pub mod pci;
pub mod proc;
pub mod serial;
pub mod testing;
pub mod util;

pub const STACK_SIZE: u64 = 1 << 16; // Limine defaults to 16KiB

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
pub fn init_kernel() -> ! {
    unsafe {
        init_kernel_services();
    }
    x86_64::instructions::interrupts::disable();
    let mut sched = KERNEL_THREAD_SCHEDULER.get();
    for i in 0..5 {
        sched.spawn(thread_one as _);
    }
    println!("{:?}", sched);
    x86_64::instructions::interrupts::enable();
    drop(sched);
    unsafe {
        asm!("int 0x20");
    }
    loop {}
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
    hardware::MODULE.init();
    memory::MODULE.init();
    #[cfg(not(test))] // Tests don't have a display
    display::MODULE.init();
    pci::MODULE.init();
    proc::MODULE.init();
    info!("Kernel services initialized");
}

extern "C" fn thread_one() -> ! {
    let mut i = 0;
    loop {
        i += 1;
        x86_64::instructions::interrupts::disable();
        sprintln!("{}", i);
        x86_64::instructions::interrupts::enable();
    }
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
