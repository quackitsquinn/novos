use core::{
    any::type_name,
    cell::UnsafeCell,
    fmt::Debug,
    mem::{self},
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicI64, AtomicUsize, Ordering},
};

use spin::Once;

use crate::core_id;

/// A readers-writer lock that can be initialized once.
pub struct OnceRwLock<T> {
    data: Once<UnsafeCell<T>>,
    readers: AtomicUsize,
    writers: AtomicUsize,
    active_writer: AtomicI64,
}

impl<T> OnceRwLock<T> {
    /// Creates a new empty `OnceRwLock`.
    pub const fn new() -> Self {
        Self {
            data: Once::new(),
            readers: AtomicUsize::new(0),
            writers: AtomicUsize::new(0),
            active_writer: AtomicI64::new(-1),
        }
    }

    /// Creates a new `OnceRwLock` initialized with the provided data.
    pub const fn initialized(data: T) -> Self {
        Self {
            data: Once::initialized(UnsafeCell::new(data)),
            readers: AtomicUsize::new(0),
            writers: AtomicUsize::new(0),
            active_writer: AtomicI64::new(-1),
        }
    }

    /// Sets the value of self to the result of the provided function.
    pub fn init(&self, init: impl FnOnce() -> T) {
        self.data.call_once(|| UnsafeCell::new(init()));
    }

    #[track_caller]
    fn get_cell(&self) -> &UnsafeCell<T> {
        match self.data.get() {
            Some(cell) => cell,
            None => panic!("OnceRwLock<{}> not initialized", type_name::<T>()),
        }
    }

    /// Acquires a write lock, blocking the current thread until it is able to do so.
    #[track_caller]
    pub fn write(&self) -> OnceRwWriteGuard<'_, T> {
        let cid = core_id();

        // If we are already the active writer, return a guard.
        if self.active_writer.load(Ordering::Relaxed) == cid as i64 {
            // if rw < 0 then we are already writing
            self.writers.fetch_add(1, Ordering::Acquire);
            return unsafe {
                OnceRwWriteGuard::from_raw_parts(
                    self.get_cell(),
                    &self.readers,
                    &self.writers,
                    &self.active_writer,
                )
            };
        }

        while self
            .active_writer
            .compare_exchange(-1, cid as i64, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        self.writers.fetch_add(1, Ordering::Acquire);

        unsafe {
            OnceRwWriteGuard::from_raw_parts(
                self.get_cell(),
                &self.readers,
                &self.writers,
                &self.active_writer,
            )
        }
    }

    /// Acquires a read lock, blocking the current thread until it is able to do so.
    pub fn read(&self) -> OnceRwReadGuard<'_, T> {
        // Spin until there is no active writer unless we are the active writer.
        let cid = core_id();

        while self.active_writer.load(Ordering::Acquire) != -1
            && self.active_writer.load(Ordering::Acquire) != cid as i64
        {
            core::hint::spin_loop();
        }

        self.readers.fetch_add(1, Ordering::Acquire);

        unsafe {
            OnceRwReadGuard::from_raw_parts(
                self.get_cell(),
                &self.readers,
                &self.writers,
                &self.active_writer,
            )
        }
    }
}

unsafe impl<T> Send for OnceRwLock<T> {}
unsafe impl<T> Sync for OnceRwLock<T> {}

/// A guard that releases the write lock when dropped.
pub struct OnceRwWriteGuard<'a, T> {
    data: &'a UnsafeCell<T>,
    readers: &'a AtomicUsize,
    writers: &'a AtomicUsize,
    active_writer: &'a AtomicI64,
}

impl<'a, T> OnceRwWriteGuard<'a, T> {
    unsafe fn from_raw_parts(
        data: &'a UnsafeCell<T>,
        readers: &'a AtomicUsize,
        writers: &'a AtomicUsize,
        active_writer: &'a AtomicI64,
    ) -> Self {
        Self {
            data,
            readers,
            writers,
            active_writer,
        }
    }

    /// Downgrades a write lock into a read lock.
    pub fn downgrade(self) -> OnceRwReadGuard<'a, T> {
        self.readers.fetch_add(1, Ordering::Release);

        let read_guard = unsafe {
            OnceRwReadGuard::from_raw_parts(
                self.data,
                self.readers,
                self.writers,
                self.active_writer,
            )
        };

        read_guard
    }
}

impl<T> Drop for OnceRwWriteGuard<'_, T> {
    fn drop(&mut self) {
        // Are we the last writer?
        if self.writers.fetch_sub(1, Ordering::Release) == 0 {
            // There are no more active writers, there is no active core writing.
            self.active_writer.store(-1, Ordering::Release);
        }
    }
}

impl<T> Deref for OnceRwWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data.get() }
    }
}

impl<T> DerefMut for OnceRwWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data.get() }
    }
}

/// A guard that releases the read lock when dropped.

pub struct OnceRwReadGuard<'a, T> {
    data: &'a UnsafeCell<T>,
    readers: &'a AtomicUsize,
    writers: &'a AtomicUsize,
    active_writer: &'a AtomicI64,
}

impl<'a, T> OnceRwReadGuard<'a, T> {
    unsafe fn from_raw_parts(
        data: &'a UnsafeCell<T>,
        readers: &'a AtomicUsize,
        writers: &'a AtomicUsize,
        active_writer: &'a AtomicI64,
    ) -> Self {
        Self {
            data,
            readers,
            writers,
            active_writer,
        }
    }

    /// Upgrades a read lock to a write lock.
    pub fn upgrade(self) -> OnceRwWriteGuard<'a, T> {
        // Check if there are other active readers.
        // This does introduce a possible single threaded deadlock.
        while let Err(_) = self
            .readers
            .compare_exchange(1, 0, Ordering::Acquire, Ordering::Relaxed)
        {
            core::hint::spin_loop();
        }

        self.writers.fetch_add(1, Ordering::Release);

        // We are officially a writer now.
        let write_guard = unsafe {
            OnceRwWriteGuard::from_raw_parts(
                self.data,
                self.readers,
                self.writers,
                self.active_writer,
            )
        };

        // Make sure none of the drop code runs.
        mem::forget(self);

        write_guard
    }
}

impl<T> Drop for OnceRwReadGuard<'_, T> {
    fn drop(&mut self) {
        self.readers.fetch_sub(1, Ordering::Release);
    }
}

impl<T> Deref for OnceRwReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data.get() }
    }
}

impl<T: Debug> Debug for OnceRwReadGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", unsafe { &*self.data.get() })
    }
}

impl<T: Debug> Debug for OnceRwWriteGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", unsafe { &*self.data.get() })
    }
}

impl<T> Debug for OnceRwLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OnceRwLock")
            .field("has_writer", &(self.writers.load(Ordering::Acquire) > 0))
            .field("readers", &self.readers.load(Ordering::Acquire))
            .finish()
    }
}

#[cfg(test)]
mod tests {

    use std::{sync::Arc, thread::spawn};

    use super::*;

    #[test]
    fn test_rr() {
        let rw = OnceRwLock::new();
        rw.init(|| 1i32);

        let r1 = rw.read();
        let r2 = rw.read();
        assert_eq!(&*r1, &*r2, "Guards are not equal")
    }

    #[test]
    fn test_wr() {
        let rw = OnceRwLock::new();
        rw.init(|| 1i32);

        let mut w = rw.write();
        let r = rw.read();
        *w = 3;
        assert_eq!(&*w, &*r)
    }

    #[test]
    fn test_wr_upgrade_w_thread() {
        let rw = Arc::new(OnceRwLock::new());
        rw.init(|| 1i32);

        let mut w = rw.write();

        let rw_thread = rw.clone();

        let thread = spawn(move || {
            let r = rw_thread.read();
            assert_eq!(*r, 3);
            println!("Read value: {}, waiting for upgrade", *r);
            let mut w = OnceRwReadGuard::upgrade(r);
            println!("Upgraded to write guard");
            *w = 4;
        });

        *w = 3;

        drop(w);
        thread.join().expect("join failed");
        let r = rw.read();
        assert_eq!(*r, 4)
    }

    #[test]
    fn test_w_downgrade_r() {
        let rw = Arc::new(OnceRwLock::new());
        rw.init(|| 1i32);

        let mut w = rw.write();
        *w = 3;
        let r = w.downgrade();
        assert_eq!(*r, 3);
    }

    #[test]
    fn test_ww() {
        let rw = Arc::new(OnceRwLock::new());
        rw.init(|| 1i32);

        let mut w1 = rw.write();
        let w2 = rw.write();

        *w1 = 2;
        assert_eq!(*w2, 2)
    }
}
