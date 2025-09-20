use alloc::{alloc::alloc, collections::btree_map::BTreeMap, vec::Vec};
use core::{
    alloc::Layout,
    arch::{asm, naked_asm},
    sync::atomic::Ordering,
};

use cake::limine::mp::Cpu;
use log::info;

use crate::mp::mp_setup::{CoreContext, CORES};

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

pub(super) fn prepare_cpu(cpu: &Cpu) {
    // First, allocate a context for the CPU.
    let context = unsafe { alloc(Layout::new::<CoreContext>()) } as *mut CoreContext;
    unsafe {
        context.write(CoreContext::new(cpu));
    }

    // Set it to the CPU's extra field and insert it into the global map.
    cpu.extra.store(context as u64, Ordering::SeqCst);
    CORES.write().insert(cpu.lapic_id, unsafe { &*context });

    cpu.goto_address.write(_ap_trampoline);
}
