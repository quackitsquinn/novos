use core::convert::Infallible;

use log::info;
use raw_cpuid::CpuId;

use crate::{
    declare_module,
    mp::{lapic::Lapic, mp_setup::dispatch_all},
};

mod lapic;
mod req_data;

pub mod mp_setup;

pub use req_data::{ApplicationCore, ApplicationCores};

pub static LAPIC: Lapic = Lapic::new();

pub fn init() -> Result<(), Infallible> {
    LAPIC.init();
    dispatch_all(apic_init);
    Ok(())
}

pub fn apic_init() {
    info!("Initializing LAPIC on core {}", current_core_id());
}

declare_module!("MP", init);

pub fn current_core_id() -> u64 {
    CpuId::new()
        .get_feature_info()
        .map_or(0, |finfo| finfo.initial_local_apic_id() as u64)
}
