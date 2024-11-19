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

use limine::request::StackSizeRequest;
use log::info;
pub(crate) use spin::Once;

pub mod display;
mod gdt;
pub mod interrupts;
pub mod memory;
pub mod panic;
pub mod serial;
pub mod testing;
mod util;

const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Because we need a relatively big stack for the display, we need to request a bigger stack size
/// from the bootloader.
// TODO: For some ungodly reason, increasing this causes something to go wrong with the framebuffer??.
// I have some probably incorrect theories about why this is happening.
// My guess is that the large stack size goes into the framebuffer info struct, which then makes it corrupted.
// I have not done enough research to confirm this, but it's my best guess.
const STACK_SIZE: u64 = 0xFF000; // 0xCF8
#[used]
static STACK_REQUEST: StackSizeRequest = StackSizeRequest::new().with_size(STACK_SIZE);

static STACK_BASE: Once<usize> = Once::new();

pub fn stack_ptr() -> usize {
    // We could also just do addr_of!(local_var), but this is more fun.
    let stack_ptr: usize;
    unsafe {
        asm!("mov {}, rsp", out(reg) stack_ptr);
    }
    stack_ptr
}
/// Check function to see how much stack space is used. Mainly used as a concrete way to check if a bug is occurring due to a stack overflow.
pub fn stack_check() -> usize {
    let base = *unsafe { STACK_BASE.get_unchecked() };
    let size = *unsafe { STACK_BASE.get_unchecked() } - stack_ptr();
    info!(
        "Stack size: 0x{:x} ({} percent) (sb: 0x{:x}, sp: 0x{:x})",
        size,
        size as f64 / STACK_SIZE as f64 * 100.0,
        base,
        stack_ptr()
    );
    size
}

// sb: 0xffff80003f74beb0,sp:0xffff80003f74c770
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
    // NOTE: This is a hack to get the stack base address.
    // This is used because limine does not provide the stack base address.
    let mut _dummy = 0;
    _dummy = &raw const _dummy as usize;
    STACK_BASE.call_once(|| _dummy);
    serial::init();
    info!("Initialized serial");
    panic::init();
    info!("Initialized panic");
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
    #[cfg(not(test))] // Tests don't have a display
    {
        info!("Initializing display");
        display::init();
        DISPLAY_INITIALIZED.call_once(|| ());
    }
    info!("Kernel initialized");

    let _ = debug_release_select!(
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
