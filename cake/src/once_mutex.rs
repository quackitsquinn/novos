use core::panic;

use log::debug;
use spin::{Mutex, MutexGuard, Once};

use crate::get_caller_rip;

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

    pub fn try_get(&self) -> Option<MutexGuard<T>> {
        self.inner.get()?.try_lock()
    }

    pub fn mutex(&self) -> &'a Mutex<T> {
        unsafe { &*(self.inner.get().unwrap() as *const Mutex<T>) }
    }
    #[track_caller]
    pub fn get(&self) -> MutexGuard<T> {
        let i = self
            .inner
            .get()
            .expect("Attempted to get an uninitialized OnceMutex!");
        if let Some(i) = i.try_lock() {
            *self.caller.lock() = get_caller_rip();
            return i;
        }
        let caller = self.caller.lock();

        if !crate::CALLER_INSTRUCTION_POINTER_FN.is_completed() {
            debug!("No caller instruction pointer function set, using default");
            panic!("OnceMutex locked by unknown caller!");
        }

        if let Some(caller) = *caller {
            let mut sym = "unknown";
            if !crate::CALLER_INSTRUCTION_POINTER_NAME_RESOLVER.is_completed() {
                sym = "unknown (no resolver set)";
            } else if let Some(name) = crate::resolve_symbol(caller) {
                sym = name;
            }
            panic!(
                "OnceMutex locked by caller at address {:p} ({})",
                caller, sym
            );
        } else {
            panic!("OnceMutex locked by unknown caller!");
        }
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

    pub unsafe fn force_get(&self) -> MutexGuard<T> {
        unsafe {
            self.force_unlock();
        }
        let t = self.get();
        // Set the caller to the correct value
        *self.caller.lock() = get_caller_rip();
        t
    }
}

unsafe impl<T> Sync for OnceMutex<T> {}

unsafe impl<T> Send for OnceMutex<T> {}
