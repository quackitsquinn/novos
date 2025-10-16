use core::panic;

use spin::{Mutex, MutexGuard, Once};

use crate::{get_caller_rip_1_up, get_caller_rip_2_up, is_multithreaded};

/// A mutex that can be initialized once.
pub struct OnceMutex<T> {
    /// The inner mutex.
    pub inner: Once<Mutex<T>>,
    caller: Mutex<Option<*const ()>>,
}

impl<'a, T> OnceMutex<T> {
    /// Creates a new uninitialized `OnceMutex`.
    pub const fn uninitialized() -> Self {
        Self {
            inner: Once::new(),
            caller: Mutex::new(None),
        }
    }

    /// Creates an already initialized `OnceMutex`.
    pub const fn new_with(value: T) -> Self {
        Self {
            inner: Once::initialized(Mutex::new(value)),
            caller: Mutex::new(None),
        }
    }

    /// Initializes it with the given value
    #[deprecated(note = "Use call_init instead for lazy evaluation of T")]
    pub fn init(&self, value: T) {
        self.inner.call_once(|| Mutex::new(value));
    }

    /// Initializes the OnceMutex with the result of the provided function.
    pub fn call_init(&self, f: impl FnOnce() -> T) {
        self.inner.call_once(|| Mutex::new(f()));
    }

    /// Tries to get the lock without blocking.
    pub fn try_get(&self) -> Option<MutexGuard<'_, T>> {
        self.inner.get()?.try_lock()
    }

    /// Gets a reference to the inner mutex.
    pub fn mutex(&'a self) -> &'a Mutex<T> {
        self.inner
            .get()
            .expect("Attempted to access an uninitialized OnceMutex!")
    }

    /// Gets a lock guard to the inner data.
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

    /// Returns if the mutex is currently locked.
    ///
    /// This does not have any synchronization guarantees.
    pub fn is_locked(&self) -> bool {
        self.mutex().is_locked()
    }

    /// REturns if the once mutex is initialized.
    pub fn is_initialized(&self) -> bool {
        self.inner.is_completed()
    }

    /// Force unlocks the mutex.
    ///
    /// # Safety
    /// This is incredibly unsafe if the mutex is locked by another thread.
    pub unsafe fn force_unlock(&self) {
        unsafe { self.mutex().force_unlock() }
    }

    /// Forces a lock guard to the inner data.
    ///
    /// # Safety
    /// This is incredibly unsafe if the mutex is locked by another thread.
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

impl<T> core::fmt::Debug for OnceMutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OnceMutex")
            .field("is_initialized", &self.is_initialized())
            .field("is_locked", &self.is_locked())
            .finish()
    }
}
