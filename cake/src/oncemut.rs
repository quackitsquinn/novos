use core::{
    cell::UnsafeCell,
    panic, ptr,
    sync::atomic::{AtomicI64, AtomicPtr, AtomicU64, AtomicUsize, Ordering},
};

use log::{error, trace};
use spin::{Mutex, MutexGuard, Once};

use crate::{get_caller_rip_1_up, resolve_symbol};

/// A mutex that can be initialized once.
pub struct OnceMutex<T> {
    /// The inner mutex.
    inner: Once<UnsafeCell<T>>,
    /// (core id / lock holder, caller instruction pointer)
    locker: (AtomicI64, AtomicPtr<()>),
}

impl<'a, T> OnceMutex<T> {
    /// Creates a new uninitialized `OnceMutex`.
    pub const fn uninitialized() -> Self {
        Self {
            inner: Once::new(),
            locker: (AtomicI64::new(-1), AtomicPtr::new(ptr::null_mut())),
        }
    }

    /// Creates an already initialized `OnceMutex`.
    pub const fn new_with(value: T) -> Self {
        Self {
            inner: Once::initialized(UnsafeCell::new(value)),
            locker: (AtomicI64::new(-1), AtomicPtr::new(ptr::null_mut())),
        }
    }

    /// Initializes the OnceMutex with the result of the provided function.
    pub fn call_init(&self, f: impl FnOnce() -> T) {
        self.inner.call_once(|| UnsafeCell::new(f()));
    }

    /// Tries to get the lock without blocking.
    pub fn try_get(&self) -> Option<OnceMutexGuard<'_, T>> {
        let cid = crate::core_id() as i64;
        let caller = get_caller_rip_1_up();

        self.acquire(cid as u64, caller)?;

        unsafe { Some(OnceMutexGuard::from_raw_parts(self.cell(), &self.locker)) }
    }

    /// Gets a reference to the inner mutex.
    fn cell(&'a self) -> &'a UnsafeCell<T> {
        self.inner
            .get()
            .expect("Attempted to access an uninitialized OnceMutex!")
    }

    /// Acquires the lock for the given core id and caller instruction pointer. Returns None on deadlock.
    fn acquire(&self, cid: u64, caller: Option<*const ()>) -> Option<()> {
        let ptr = caller.unwrap_or_else(ptr::dangling).cast_mut();
        let state =
            self.locker
                .0
                .compare_exchange(-1, cid as i64, Ordering::AcqRel, Ordering::Acquire);

        // If we failed to acquire the lock, check for deadlock, then spin until we acquire it.
        if let Err(locker_cid) = state {
            self.lock_check(locker_cid, cid)?;
            while self
                .locker
                .0
                .compare_exchange(-1, cid as i64, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {}
        }

        // Set the caller pointer
        self.locker.1.store(ptr, Ordering::Release);
        Some(())
    }

    fn lock_check(&self, locker_cid: i64, cid: u64) -> Option<()> {
        if locker_cid != cid as i64 {
            return Some(());
        }

        let locker_caller = self.locker.1.load(Ordering::Acquire);

        if locker_caller.is_null() {
            trace!(
                "Deadlock detected: Attempted to re-lock OnceMutex on core {} by unknown!",
                cid
            );
            return None;
        }

        if let Some(s) = resolve_symbol(locker_caller) {
            trace!(
                "Deadlock detected: Attempted to re-lock OnceMutex on core {}: Locked by {}.",
                cid, s,
            );
            return None;
        }

        trace!(
            "Deadlock detected: Attempted to re-lock OnceMutex on core {} by unknown!",
            cid
        );

        None
    }

    /// Gets a lock guard to the inner data.
    #[track_caller]
    pub fn get(&self) -> OnceMutexGuard<'_, T> {
        let caller = get_caller_rip_1_up();
        let cid = crate::core_id() as i64;

        self.acquire(cid as u64, caller)
            .expect("Deadlock detected on OnceMutex!");

        unsafe { OnceMutexGuard::from_raw_parts(self.cell(), &self.locker) }
    }

    /// Returns if the mutex is currently locked.
    ///
    /// This does not have any synchronization guarantees.
    pub fn is_locked(&self) -> bool {
        self.locker.0.load(Ordering::Acquire) != -1
    }

    /// Returns true if the mutex has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.inner.is_completed()
    }
}

unsafe impl<T> Sync for OnceMutex<T> {}

unsafe impl<T> Send for OnceMutex<T> {}

impl<T> core::fmt::Debug for OnceMutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OnceMutex")
            .field("is_initialized", &self.is_initialized())
            .field("is_locked", &self.is_locked())
            .finish()
    }
}

/// A guard that releases the OnceMutex when dropped.
#[derive(Debug)]
pub struct OnceMutexGuard<'a, T> {
    guard: &'a UnsafeCell<T>,
    locker: &'a (AtomicI64, AtomicPtr<()>),
}

impl<'a, T> OnceMutexGuard<'a, T> {
    unsafe fn from_raw_parts(
        guard: &'a UnsafeCell<T>,
        locker: &'a (AtomicI64, AtomicPtr<()>),
    ) -> Self {
        Self { guard, locker }
    }
}

impl<T> Drop for OnceMutexGuard<'_, T> {
    fn drop(&mut self) {
        // Clear the locker info
        self.locker.1.store(ptr::null_mut(), Ordering::Release);
        self.locker.0.store(-1, Ordering::Release);
    }
}

impl<T> core::ops::Deref for OnceMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.guard.get() }
    }
}

impl<T> core::ops::DerefMut for OnceMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.guard.get() }
    }
}
