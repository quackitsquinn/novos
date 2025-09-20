pub mod context;
pub mod trampoline;

use core::convert::Infallible;

use alloc::collections::btree_map::BTreeMap;
use cake::{declare_module, spin::RwLock};
pub use context::CoreContext;
use log::info;

use crate::{mp::mp_setup::trampoline::prepare_cpu, requests::MP_INFO};

pub static CORES: RwLock<BTreeMap<u32, &'static CoreContext>> = RwLock::new(BTreeMap::new());

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

declare_module!("MP Preinit", init);
