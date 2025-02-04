use core::arch::asm;

use log::info;
use x86_64::structures::idt::InterruptStackFrame;

mod int_ctx;

pub extern "C" fn ctx_test(ptr: *mut int_ctx::InterruptContext) {
    let registers = unsafe { &mut *ptr };
    info!("Interrupt registers: {:?}", registers);
}

pub use int_ctx::ctx_test_raw;
