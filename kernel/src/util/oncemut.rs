use core::panic;

use spin::{Mutex, MutexGuard, Once};

pub struct OnceMutex<T> {
    pub inner: Once<Mutex<T>>,
}

impl<'a, T> OnceMutex<T> {
    pub const fn new() -> Self {
        Self { inner: Once::new() }
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
        if let Some(i) = i.try_lock() {
            return i;
        }
        panic!("Attempted to lock a locked mutex!");
    }

    pub fn is_locked(&self) -> bool {
        self.mutex().is_locked()
    }

    pub unsafe fn force_unlock(&self) {
        unsafe { self.mutex().force_unlock() }
    }
}
