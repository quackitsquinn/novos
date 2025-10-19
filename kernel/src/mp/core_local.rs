use core::{fmt::Debug, mem};

use alloc::vec::Vec;
use cake::{Once, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::mp::mp_setup::{CORES, trampoline::aps_finished};

/// A structure that holds data local to each core.
#[derive(Debug)]
pub struct CoreLocal<T, C = Constructor<T>> {
    bootstrap: RwLock<T>,
    ctor: C,
    /// (APIC ID, Data)
    applications: Once<Vec<(u64, RwLock<T>)>>,
}

impl<T, C: ConstructMethod<T>> CoreLocal<T, C> {
    /// Create a new CoreLocal structure.
    ///
    /// # Arguments
    /// * `bootstrap` - The data for the bootstrap core (core 0).
    /// * `ctor` - A constructor function to create data for application cores.
    pub const fn new(bootstrap: T, ctor: C) -> Self {
        CoreLocal {
            bootstrap: RwLock::new(bootstrap),
            ctor: ctor,
            applications: Once::new(),
        }
    }

    fn create_instance(&self) -> T {
        self.ctor.construct(&self)
    }

    fn get_or_init_applications(&self) -> &[(u64, RwLock<T>)] {
        if let Some(apps) = self.applications.get() {
            return apps;
        }

        if !aps_finished() {
            panic!("Attempted to access application core data before all cores initialized");
        }
        self.applications.call_once(|| {
            let cores = CORES.get().expect("Cores not initialized");
            let mut apps = Vec::with_capacity(cores.len() - 1);
            for (&apic_id, _) in cores.iter().filter(|(id, _)| **id != 0) {
                apps.push((apic_id as u64, RwLock::new(self.create_instance())));
            }

            apps.sort_by_key(|(id, _)| *id);

            apps
        })
    }

    /// Get a reference to the data for the current core.
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
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
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
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
}

trait Sealed {}

pub trait ConstructMethod<T>: Sealed + Sized {
    fn construct(&self, local: &CoreLocal<T, Self>) -> T;
}

/// A construct method that uses a constructor function to create data for application cores.
pub struct Constructor<T>(pub fn() -> T);
impl<T> Sealed for Constructor<T> {}

impl<T> Debug for Constructor<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Ctor").finish()
    }
}

impl<T> Constructor<T> {
    /// Create a new Ctor construct method.
    pub const fn new(f: fn() -> T) -> Self {
        Constructor(f)
    }
}

impl<T> ConstructMethod<T> for Constructor<T> {
    fn construct(&self, _local: &CoreLocal<T, Self>) -> T {
        (self.0)()
    }
}

/// A construct method that clones the bootstrap core's data for application cores.
pub struct CloneBootstrap<T>(core::marker::PhantomData<T>);
impl<T: Clone> Sealed for CloneBootstrap<T> {}

impl<T: Clone> Debug for CloneBootstrap<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CloneBootstrap").finish()
    }
}

impl<T> CloneBootstrap<T> {
    /// Create a new CloneBootstrap construct method.
    pub const fn new() -> Self {
        CloneBootstrap(core::marker::PhantomData)
    }
}

impl<T: Clone> ConstructMethod<T> for CloneBootstrap<T> {
    fn construct(&self, local: &CoreLocal<T, Self>) -> T {
        local.bootstrap.read().clone()
    }
}
