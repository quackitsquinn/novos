use core::ops::{Deref, DerefMut};

use cake::spin::Mutex;

use crate::interrupts;

/// A mutex that disables interrupts while locked.
pub struct InterruptMutex<T> {
    data: Mutex<T>,
}

/// A guard that releases the interrupt mutex when dropped.
pub struct InterruptMutexGuard<'a, T> {
    guard: cake::spin::MutexGuard<'a, T>,
    reenable: bool,
}

impl<T> Drop for InterruptMutexGuard<'_, T> {
    fn drop(&mut self) {
        if self.reenable {
            interrupts::enable();
        }
    }
}

impl<T> Deref for InterruptMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<T> DerefMut for InterruptMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}

impl<T> InterruptMutex<T> {
    /// Creates a new `InterruptMutex`.
    pub const fn new(data: T) -> Self {
        InterruptMutex {
            data: Mutex::new(data),
        }
    }

    /// Locks the mutex, disabling interrupts.
    pub fn lock(&self) -> InterruptMutexGuard<'_, T> {
        let reenable = interrupts::are_enabled();
        interrupts::disable();
        let guard = self.data.lock();
        InterruptMutexGuard { guard, reenable }
    }
}

unsafe impl<T: Send> Sync for InterruptMutex<T> {}
unsafe impl<T: Send> Send for InterruptMutex<T> {}
