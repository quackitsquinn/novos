use core::alloc::{AllocError, Allocator};
use core::ptr::NonNull;
use core::{
    alloc::{GlobalAlloc, Layout},
    fmt::Debug,
};

use cake::spin::{Mutex, MutexGuard, Once};

use crate::mut_alloc::MutableAllocator;

/// A Send + Sync wrapper around a global allocator. This is safe for statics.
pub struct GlobalAllocatorWrapper<T>
where
    T: MutableAllocator,
{
    inner: Once<Mutex<T>>,
}

impl<T> GlobalAllocatorWrapper<T>
where
    T: MutableAllocator,
{
    /// Creates a new global allocator wrapper. The allocator is uninitialized.
    pub const fn new() -> Self {
        Self { inner: Once::new() }
    }
    /// Initializes the global allocator with the given function.
    pub fn init<F>(&self, init: F)
    where
        F: FnOnce() -> T,
    {
        self.inner.call_once(|| Mutex::new(init()));
    }
    /// Gets the global allocator, if it is initialized.
    pub fn get(&self) -> Option<MutexGuard<'_, T>> {
        self.inner.get()?.try_lock()
    }
    /// Is the global allocator locked?
    pub fn is_locked(&self) -> bool {
        self.inner.get().is_none() || self.inner.get().as_ref().unwrap().is_locked()
    }
    /// Is the global allocator initialized?
    pub fn is_initialized(&self) -> bool {
        self.inner.is_completed()
    }

    /// Forces the global allocator to unlock.
    pub unsafe fn force_unlock(&self) {
        if let Some(inner) = self.inner.get() {
            unsafe { inner.force_unlock() };
        }
    }

    /// Force unlocks the global allocator and returns it.
    pub unsafe fn force_get(&self) -> Option<MutexGuard<'_, T>> {
        if let Some(inner) = self.inner.get() {
            unsafe {
                inner.force_unlock();
                Some(inner.lock())
            }
        } else {
            None
        }
    }
}

unsafe impl<T> GlobalAlloc for GlobalAllocatorWrapper<T>
where
    T: MutableAllocator,
{
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if let Some(mut alloc) = self.get() {
            unsafe { alloc.alloc(layout) }
        } else {
            if self.inner.get().is_none() {
                panic!("Attempted to allocate with an uninitialized global allocator");
            }
            // This just means that the mutex is locked, so we can't allocate. This will probably crash down the line, but it ain't our problem.
            aerror!("Attempted to allocate with a locked global allocator");
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if let Some(mut alloc) = self.get() {
            unsafe {
                alloc.dealloc(ptr, layout);
            }
        } else {
            if self.inner.get().is_none() {
                panic!("Attempted to deallocate with an uninitialized global allocator");
            }
            // This just means that the mutex is locked, so we can't allocate.
            aerror!("Attempted to deallocate with a locked global allocator");
        }
    }
}

unsafe impl<T> Allocator for GlobalAllocatorWrapper<T>
where
    T: MutableAllocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() == 0 {
            return Ok(NonNull::slice_from_raw_parts(
                core::ptr::NonNull::dangling(),
                0,
            ));
        }
        let ptr = unsafe { self.alloc(layout) };
        if ptr.is_null() {
            Err(AllocError)
        } else {
            Ok(NonNull::slice_from_raw_parts(
                NonNull::new(ptr).unwrap(),
                layout.size(),
            ))
        }
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        if layout.size() == 0 {
            return;
        }
        unsafe { self.dealloc(ptr.as_ptr(), layout) };
    }
}

impl<T> Debug for GlobalAllocatorWrapper<T>
where
    T: MutableAllocator + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(alloc) = self.get() {
            alloc.fmt(f)
        } else {
            write!(f, "GlobalAllocatorWrapper {{ <locked> }}")
        }
    }
}

// I believe this is safe because all accesses are protected by a mutex.
unsafe impl<T> Send for GlobalAllocatorWrapper<T> where T: MutableAllocator {}
unsafe impl<T> Sync for GlobalAllocatorWrapper<T> where T: MutableAllocator {}
