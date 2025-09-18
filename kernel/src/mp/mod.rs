use core::{
    alloc::Layout,
    convert::Infallible,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
};

use alloc::{alloc::alloc, collections::btree_map::BTreeMap, vec::Vec};
use cake::{
    limine::{mp::Cpu, request::MpRequest, response::MpResponse},
    spin::{Mutex, RwLock},
};
use log::info;
use raw_cpuid::CpuId;

use crate::{declare_module, println, requests::MP_INFO};

mod req_data;
mod trampoline;

pub use req_data::{ApplicationCore, ApplicationCores};

pub static CORES: RwLock<BTreeMap<u32, &'static CoreContext>> = RwLock::new(BTreeMap::new());

pub struct CoreContext {
    todo: u32,
}

impl CoreContext {
    const fn new(cpu: &Cpu) -> Self {
        Self { todo: 0 }
    }
}

pub fn init() -> Result<(), Infallible> {
    let mp = MP_INFO.get_limine();

    let cpus = mp.cpus();

    let ap_cpus = cpus.len() - 1;
    info!("Found {} apCPUs", ap_cpus);

    cake::set_multithreaded(ap_cpus > 0);

    for cpu in cpus {
        info!("Initializing  CPU {} (APIC ID {})", cpu.id, cpu.lapic_id);
        if cpu.id == 0 {
            continue; // Skip BSP
        }
        prepare_cpu(cpu);
        info!("Prepared CPU {} (APIC ID {})", cpu.id, cpu.lapic_id);
    }

    Ok(())
}

fn prepare_cpu(cpu: &Cpu) {
    // First, allocate a context for the CPU.
    let context = unsafe { alloc(Layout::new::<CoreContext>()) } as *mut CoreContext;
    unsafe {
        context.write(CoreContext::new(cpu));
    }

    // Set it to the CPU's extra field and insert it into the global map.
    cpu.extra.store(context as u64, Ordering::SeqCst);
    CORES.write().insert(cpu.lapic_id, unsafe { &*context });

    cpu.goto_address.write(trampoline::_ap_trampoline);
}

declare_module!("MP", init);

pub fn current_core_id() -> u64 {
    CpuId::new()
        .get_feature_info()
        .map_or(0, |finfo| finfo.initial_local_apic_id() as u64)
}
