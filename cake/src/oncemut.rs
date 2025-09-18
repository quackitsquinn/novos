use core::panic;

use log::debug;
use spin::{Mutex, MutexGuard, Once};

use crate::{get_caller_rip_1_up, get_caller_rip_2_up, is_multithreaded};

pub struct OnceMutex<T> {
    pub inner: Once<Mutex<T>>,
    caller: Mutex<Option<*const ()>>,
}

impl<'a, T> OnceMutex<T> {
    pub const fn uninitialized() -> Self {
        Self {
            inner: Once::new(),
            caller: Mutex::new(None),
        }
    }

    pub const fn new_with(value: T) -> Self {
        Self {
            inner: Once::initialized(Mutex::new(value)),
            caller: Mutex::new(None),
        }
    }

    pub fn init(&self, value: T) {
        self.inner.call_once(|| Mutex::new(value));
    }

    pub fn try_get(&self) -> Option<MutexGuard<'_, T>> {
        self.inner.get()?.try_lock()
    }

    pub fn mutex(&'a self) -> &'a Mutex<T> {
        self.inner
            .get()
            .expect("Attempted to access an uninitialized OnceMutex!")
    }

    #[track_caller]
    pub fn get(&self) -> MutexGuard<'_, T> {
        if is_multithreaded() {
            self.get_multithreaded()
        } else {
            self.get_singlethreaded()
        }
    }

    fn get_singlethreaded(&self) -> MutexGuard<'_, T> {
        let mutex = self.mutex();
        if !mutex.is_locked() {
            *self.caller.lock() = get_caller_rip_2_up!();
            return mutex.lock();
        }

        let caller = self.caller.lock();

        if caller.is_none() {
            panic!("OnceMutex locked by unknown caller!");
        }

        let caller = caller.unwrap();

        let mut sym = "unknown";
        if let Some(name) = crate::resolve_symbol(caller) {
            sym = name;
        }

        panic!(
            "OnceMutex locked by caller at address {:p} ({})",
            caller, sym
        );
    }

    fn get_multithreaded(&self) -> MutexGuard<'_, T> {
        self.mutex().lock()
    }

    pub fn is_locked(&self) -> bool {
        self.mutex().is_locked()
    }

    pub fn is_initialized(&self) -> bool {
        self.inner.is_completed()
    }

    pub unsafe fn force_unlock(&self) {
        unsafe { self.mutex().force_unlock() }
    }

    pub unsafe fn force_get(&self) -> MutexGuard<'_, T> {
        unsafe {
            self.force_unlock();
        }
        let t = self.get();
        // Set the caller to the correct value
        *self.caller.lock() = get_caller_rip_1_up();
        t
    }
}

unsafe impl<T> Sync for OnceMutex<T> {}

unsafe impl<T> Send for OnceMutex<T> {}
