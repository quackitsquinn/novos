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
    pub fn get(&self) -> MutexGuard<T> {
        let i = self.inner.get().unwrap();
        if let Some(i) = i.try_lock() {
            return i;
        }
        panic!("Attempted to lock a locked mutex!");
    }
}
