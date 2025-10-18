pub mod context;
pub mod trampoline;

use core::convert::Infallible;

use alloc::collections::btree_map::BTreeMap;
use cake::log::info;
use cake::{Once, RwLock, declare_module};
pub use context::CoreContext;

use crate::{mp::mp_setup::trampoline::prepare_cpu, requests::MP_INFO};

static CORES: Once<BTreeMap<u32, &'static RwLock<CoreContext>>> = Once::new();

pub(super) fn init() -> Result<(), Infallible> {
    let mp = MP_INFO.get_limine();

    let cpus = mp.cpus();

    let ap_cpus = cpus.len() - 1;
    info!("Found {} apCPUs", ap_cpus);

    cake::set_multithreaded(ap_cpus > 0);

    let mut cores = BTreeMap::new();

    for cpu in cpus {
        info!("Initializing  CPU {} (APIC ID {})", cpu.id, cpu.lapic_id);
        if cpu.id == 0 {
            continue; // Skip BSP
        }
        let context = unsafe { &*prepare_cpu(cpu) };
        cores.insert(cpu.lapic_id, context);
        info!("Prepared CPU {} (APIC ID {})", cpu.id, cpu.lapic_id);
    }

    CORES.call_once(|| cores);

    Ok(())
}

/// Returns a reference to the map of all core contexts.
pub fn cores() -> &'static BTreeMap<u32, &'static RwLock<CoreContext>> {
    CORES.wait()
}

/// Dispatches a function to a specific CPU by its APIC ID.
pub fn dispatch_to(cpu_id: u32, f: fn() -> ()) -> Result<(), &'static str> {
    let cores = cores();
    let mut core = cores.get(&cpu_id).ok_or("No such core")?.write();
    core.add_task(f);
    Ok(())
}

/// Dispatches a function to all application processors and also runs it on the current processor.
pub fn dispatch_all(f: fn() -> ()) {
    let cores = cores();
    for mut core in cores.values().map(|c| c.write()) {
        core.add_task(f);
    }
    f();
}

declare_module!("MP Preinit", init);
