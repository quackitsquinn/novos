use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering},
};

use cake::{core_id, spin::Mutex};

use crate::interrupts;

// A mutex that disables interrupts while locked.
pub struct InterruptMutex<T> {
    data: UnsafeCell<T>,
    lock: AtomicBool,
    locker_thread: AtomicU32,
}

pub struct InterruptMutexGuard<'a, T> {
    data: *mut T,
    lock: &'a AtomicBool,
    reenable: bool,
}

impl<T> Drop for InterruptMutexGuard<'_, T> {
    fn drop(&mut self) {
        if self.reenable {
            interrupts::enable();
        }
        self.lock.store(false, Ordering::Release);
    }
}

impl<T> Deref for InterruptMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T> DerefMut for InterruptMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<T> InterruptMutex<T> {
    pub const fn new(data: T) -> Self {
        InterruptMutex {
            data: UnsafeCell::new(data),
            lock: AtomicBool::new(false),
            locker_thread: AtomicU32::new(0),
        }
    }

    pub fn lock(&self) -> InterruptMutexGuard<'_, T> {
        let reenable = interrupts::are_enabled();
        interrupts::disable();
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        self.locker_thread
            .store(core_id() as u32, Ordering::Release);
        InterruptMutexGuard {
            data: self.data.get(),
            lock: &self.lock,
            reenable,
        }
    }

    pub unsafe fn lock_interrupt(&self) -> InterruptMutexGuard<'_, T> {
        let reenable = interrupts::are_enabled();
        interrupts::disable();
        if self.lock.load(Ordering::Acquire)
            && self.locker_thread.load(Ordering::Acquire) == core_id() as u32
        {
            // We are in an interrupt handler on the same core that holds the lock.
            return InterruptMutexGuard {
                data: self.data.get(),
                lock: &self.lock,
                reenable,
            };
        }

        self.lock()
    }
}

unsafe impl<T: Send> Sync for InterruptMutex<T> {}
unsafe impl<T: Send> Send for InterruptMutex<T> {}
