use core::panic;

use spin::{Mutex, MutexGuard, Once};

use crate::panic::stacktrace::{fmt_symbol, get_caller_rip};

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

    pub fn get(&self) -> MutexGuard<T> {
        let i = self
            .inner
            .get()
            .expect("Attempted to get an uninitialized OnceMutex!");
        // TODO: Do some fancy stack trace stuff here and save the last lock location. Would be greatly useful for debugging.
        if let Some(i) = i.try_lock() {
            self.caller.lock().replace(get_caller_rip());
            return i;
        }
        let caller = self.caller.lock();
        if let Some(caller) = *caller {
            panic!("Mutex already locked by: {}", fmt_symbol(caller));
        } else {
            panic!("Mutex already locked by unknown location");
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
        self.caller.lock().replace(get_caller_rip());
        t
    }
}

unsafe impl<T> Sync for OnceMutex<T> {}

unsafe impl<T> Send for OnceMutex<T> {}
