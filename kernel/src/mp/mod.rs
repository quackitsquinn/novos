//! Multiprocessor setup and processor local APIC management.
use core::convert::Infallible;

use alloc::vec::Vec;
use cake::{Once, log::info};
use raw_cpuid::CpuId;

use crate::{
    declare_module,
    interrupts::hardware,
    mp::{
        ioapic::IoApic,
        lapic::Lapic,
        mp_setup::{dispatch_all, trampoline::core_wait},
    },
};

pub mod ioapic;
pub mod lapic;
pub mod req_data;

mod core_local;
mod mp_setup;

pub use mp_setup::{
    CoreContext, MODULE as PREINIT_MODULE, cores, dispatch_to, is_initialized as has_init_mp,
};

pub use core_local::CoreLocal;

pub use req_data::{ApplicationCore, ApplicationCores};

/// The local APIC for the current core.
pub static LAPIC: Lapic = Lapic::new();

/// The IO APIC for the current system.
pub static IOAPIC: IoApic = IoApic::new();

pub static CPU_IDS: Once<Vec<u64>> = Once::new();

fn init() -> Result<(), Infallible> {
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
    core_wait();
    Ok(())
}

fn apic_init() {
    info!("Initializing LAPIC on core {}", current_core_id());
}

declare_module!("MP", init);

/// Returns the current core's APIC ID.
#[inline]
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

mod _macro {
    /// Defines a constant identifier for a given type.
    macro_rules! id {
        ($typ: ident, $name: ident, $value: expr) => {
            impl $typ {
                /// The register offset for this type.
                pub const $name: usize = $value;
            }
        };
    }

    pub(super) use id;
}

pub(self) use _macro::*;
