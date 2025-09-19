use core::arch::{asm, naked_asm};

use cake::limine::mp::Cpu;
use log::info;

use crate::mp::CoreContext;

#[unsafe(naked)]
pub unsafe extern "C" fn _ap_trampoline(a: &Cpu) -> ! {
    naked_asm!(
        "mov rsi, rsp", // Pass rsp as the second argument (first is cpu pointer)
        "call {ap_trampoline}",
        ap_trampoline = sym ap_trampoline,
    )
}

// Application Processor trampoline function.
extern "C" fn ap_trampoline(cpu: &Cpu, stack_base: u64) -> ! {
    let context =
        unsafe { &*(cpu.extra.load(core::sync::atomic::Ordering::SeqCst) as *const CoreContext) };

    context.stack_start.call_once(|| stack_base);

    info!("CPU {} (APIC ID {}) started", cpu.id, cpu.lapic_id);
    info!("Stack base: {:#x}", stack_base);
    loop {
        unsafe {
            asm!("hlt");
        }
        core::hint::spin_loop();
    }
}
