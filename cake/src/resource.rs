use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};

/// An advanced mutex that provides some more quality-of-life features than spin::Mutex.
pub struct ResourceMutex<T> {
    inner: UnsafeCell<T>,
    lock: AtomicBool,
    validator: Option<fn() -> bool>,
}

impl<T> ResourceMutex<T> {
    /// Creates a new `ResourceMutex` wrapping the given data.
    pub const fn new(data: T) -> Self {
        Self {
            inner: UnsafeCell::new(data),
            lock: AtomicBool::new(false),
            validator: None,
        }
    }

    /// Sets a validator function that will be called whenever the mutex is accessed. If the validator returns false, the mutex will panic.
    /// This is useful for ensuring that certain conditions are met before accessing the mutex, such as if limine requests have not terminated.
    pub const fn with_validator(mut self, validator: fn() -> bool) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Locks the mutex without returning a guard. The caller is responsible for unlocking the mutex.
    /// # Safety
    /// The caller must ensure that they unlock the mutex after calling this function.
    pub unsafe fn lock_guardless(&self) {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    /// Forcefully unlocks the mutex.
    ///
    /// # Safety
    /// The caller must ensure that they hold the lock before calling this function.
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    /// Locks the mutex and returns a guard that allows access to the inner data.
    /// The mutex is released when the guard is dropped.
    pub fn lock(&self) -> ResourceGuard<'_, T> {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        ResourceGuard {
            data: unsafe { &mut *self.inner.get() },
            lock: &self.lock,
            validator: self.validator,
        }
    }

    /// Locks the mutex and maps the inner data to a different type using the provided closure.
    pub fn lock_map<'a, F, U>(&'a self, f: F) -> ResourceGuard<'a, U>
    where
        F: FnOnce(&'a mut T) -> &'a mut U,
    {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        ResourceGuard {
            data: f(unsafe { &mut *self.inner.get() }),
            lock: &self.lock,
            validator: self.validator,
        }
    }

    /// Returns true if the mutex is currently locked.
    pub fn is_locked(&self) -> bool {
        self.lock.load(core::sync::atomic::Ordering::SeqCst)
    }
}

unsafe impl<T: Send> Send for ResourceMutex<T> {}
unsafe impl<T: Send> Sync for ResourceMutex<T> {}

/// A guard that allows access to the inner data of a `ResourceMutex`. The mutex is released when the guard is dropped.
pub struct ResourceGuard<'a, T> {
    data: &'a mut T,
    lock: &'a AtomicBool,
    validator: Option<fn() -> bool>,
}

impl<T> Drop for ResourceGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl<T> core::ops::Deref for ResourceGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if self.validator.is_some() && !(self.validator.unwrap())() {
            panic!("ResourceMutex validator failed");
        }
        self.data
    }
}

impl<T> core::ops::DerefMut for ResourceGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.validator.is_some() && !(self.validator.unwrap())() {
            panic!("ResourceMutex validator failed");
        }
        self.data
    }
}

unsafe impl<T: Send> Send for ResourceGuard<'_, T> {}
unsafe impl<T: Sync> Sync for ResourceGuard<'_, T> {}
