#![no_std]
#![no_main]

use core::arch::asm;

pub(crate) use spin::{Mutex, Once};

pub(crate) type OnceMut<T> = Once<Mutex<T>>;

pub mod display;
pub mod serial;

/// Halts the CPU indefinitely.
pub fn hlt_loop() -> ! {
    // SAFETY: We only call cli and hlt, which are safe to call.
    unsafe { asm!("cli") };
    loop {
        unsafe { asm!("hlt") };
    }
}

macro_rules! wait_for {
    ($condition: expr) => {
        while !$condition {
            core::hint::spin_loop();
        }
    };
}

pub fn init_kernel() {
    serial::init();
}
