use core::{
    alloc::Layout,
    convert::Infallible,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
};

use alloc::{alloc::alloc, collections::btree_map::BTreeMap, vec::Vec};
use cake::{
    limine::{mp::Cpu, request::MpRequest, response::MpResponse},
    spin::{once::Once, Mutex, RwLock},
};
use log::info;
use raw_cpuid::CpuId;
use x86_64::VirtAddr;

use crate::{
    declare_module,
    mp::{lapic::Lapic, mp_setup::dispatch_all},
    println,
    requests::MP_INFO,
};

mod lapic;
mod req_data;

pub mod mp_setup;

pub use req_data::{ApplicationCore, ApplicationCores};

pub fn init() -> Result<(), Infallible> {
    fn core_hi() {
        println!("Hello from core {}", current_core_id());
    }
    dispatch_all(core_hi);
    Ok(())
}

declare_module!("MP", init);

pub fn current_core_id() -> u64 {
    CpuId::new()
        .get_feature_info()
        .map_or(0, |finfo| finfo.initial_local_apic_id() as u64)
}
