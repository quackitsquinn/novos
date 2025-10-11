use alloc::alloc::alloc;
use core::{
    alloc::Layout,
    arch::naked_asm,
    hint,
    sync::atomic::Ordering,
};
use x86_64::registers::control::{Cr3, Cr3Flags};

use cake::{limine::mp::Cpu, spin::RwLock};
use cake::log::info;

use crate::{
    interrupts::IDT,
    memory::paging::kernel::KERNEL_CR3,
    mp::mp_setup::CoreContext,
};

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
    let context = unsafe {
        &*(cpu.extra.load(core::sync::atomic::Ordering::SeqCst) as *const RwLock<CoreContext>)
    };

    unsafe { IDT.load() };

    let context_lock = context.write();

    context_lock.stack_start.call_once(|| stack_base);

    info!("CPU {} (APIC ID {}) started", cpu.id, cpu.lapic_id);
    info!("Stack base: {:#x}", stack_base);
    drop(context_lock);

    // Switch into the kernel page table as soon as possible.
    info!("Waiting for kernel page table...");
    let cr3 = KERNEL_CR3.wait();
    info!(
        "Switching to kernel page table with root frame: {:#x?}",
        cr3
    );
    // Switch to the kernel page table.
    unsafe {
        Cr3::write(*cr3, Cr3Flags::empty());
    }
    loop {
        let context_lock = context.upgradeable_read();
        if context_lock.tasks.is_empty() {
            drop(context_lock);
            hint::spin_loop();
            continue;
        }

        let mut context_lock = context_lock.upgrade();
        let task = context_lock.tasks.remove(0);
        task();
    }
}

pub(super) fn prepare_cpu(cpu: &Cpu) -> *const RwLock<CoreContext> {
    // First, allocate a context for the CPU.
    let context =
        unsafe { alloc(Layout::new::<RwLock<CoreContext>>()) } as *mut RwLock<CoreContext>;
    unsafe {
        context.write(RwLock::new(CoreContext::new(cpu)));
    }

    // Set it to the CPU's extra field and insert it into the global map.
    cpu.extra.store(context as u64, Ordering::SeqCst);

    cpu.goto_address.write(_ap_trampoline);

    context
}
