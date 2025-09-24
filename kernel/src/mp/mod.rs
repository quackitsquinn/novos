use core::{convert::Infallible, sync::atomic::AtomicU32};

use log::info;
use raw_cpuid::{CpuId, CpuIdResult};

use crate::{
    declare_module,
    interrupts::hardware,
    mp::{ioapic::IoApic, lapic::Lapic, mp_setup::dispatch_all},
};

mod ioapic;
mod lapic;
mod req_data;

pub mod mp_setup;

pub use req_data::{ApplicationCore, ApplicationCores};

pub static LAPIC: Lapic = Lapic::new();
pub static IOAPIC: IoApic = IoApic::new();

pub fn init() -> Result<(), Infallible> {
    LAPIC.init();
    IOAPIC.init();
    info!("IO APIC Version: {:?}", IOAPIC.version());
    let version = IOAPIC.version();
    info!(
        "Max Redirection Entries: {}",
        version.max_redirection_entries()
    );

    // Disable the PICs
    unsafe {
        hardware::disable();
    }
    dispatch_all(apic_init);
    Ok(())
}

pub fn apic_init() {
    info!("Initializing LAPIC on core {}", current_core_id());
}

declare_module!("MP", init);

pub fn current_core_id() -> u64 {
    // INFO: We don't use `CpuId::new()` because
    CpuId::with_cpuid_reader(raw_cpuid::CpuIdReaderNative)
        .get_feature_info()
        .map_or(0, |finfo| finfo.initial_local_apic_id() as u64)
}

fn apic_page_flags() -> x86_64::structures::paging::PageTableFlags {
    use x86_64::structures::paging::PageTableFlags as Flags;
    Flags::PRESENT | Flags::NO_CACHE | Flags::WRITABLE | Flags::NO_EXECUTE
}
