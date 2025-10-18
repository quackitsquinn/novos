use alloc::vec::Vec;
use cake::{Once, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::mp::mp_setup::{CORES, trampoline::aps_finished};

/// A structure that holds data local to each core.
#[derive(Debug)]
pub struct CoreLocal<T> {
    bootstrap: RwLock<T>,
    ctor: fn() -> T,
    /// (APIC ID, Data)
    applications: Once<Vec<(u64, RwLock<T>)>>,
}

impl<T> CoreLocal<T> {
    /// Create a new CoreLocal structure.
    ///
    /// # Arguments
    /// * `bootstrap` - The data for the bootstrap core (core 0).
    /// * `ctor` - A constructor function to create data for application cores.
    pub const fn new(bootstrap: T, ctor: fn() -> T) -> Self {
        CoreLocal {
            bootstrap: RwLock::new(bootstrap),
            ctor,
            applications: Once::new(),
        }
    }

    fn get_or_init_applications(&self) -> &[(u64, RwLock<T>)] {
        if let Some(apps) = self.applications.get() {
            return apps;
        }

        if !aps_finished() {
            panic!("Attempted to access application core data before all cores initialized");
        }

        let cores = CORES.get().expect("Cores not initialized");
        let mut apps = Vec::with_capacity(cores.len() - 1);
        for (&apic_id, _) in cores.iter().filter(|(id, _)| **id != 0) {
            apps.push((apic_id as u64, RwLock::new(self.create_instance())));
        }

        apps.sort_by_key(|(id, _)| *id);

        self.applications.call_once(|| apps)
    }

    /// Get a reference to the data for the current core.
    pub fn get(&mut self) -> RwLockReadGuard<T> {
        let core_id = crate::mp::current_core_id();
        if core_id == 0 {
            return self.bootstrap.read();
        }

        let apps = self.get_or_init_applications();
        if let Ok(ind) = apps.binary_search_by_key(&core_id, |(id, _)| *id) {
            return apps[ind].1.read();
        }
        panic!("No data for core ID {}", core_id);
    }

    /// Get a mutable reference to the data for the current core.
    pub fn get_mut(&mut self) -> RwLockWriteGuard<T> {
        let core_id = crate::mp::current_core_id();
        if core_id == 0 {
            return self.bootstrap.write();
        }

        let apps = self.get_or_init_applications();
        if let Ok(ind) = apps.binary_search_by_key(&core_id, |(id, _)| *id) {
            return apps[ind].1.write();
        }
        panic!("No data for core ID {}", core_id);
    }

    fn create_instance(&self) -> T {
        (self.ctor)()
    }
}
