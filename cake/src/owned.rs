use core::{
    fmt,
    ops::{Deref, DerefMut},
    ptr::{NonNull, drop_in_place},
};

/// A pointer type that provides ownership semantics.
#[repr(transparent)]
pub struct Owned<T: ?Sized> {
    val: NonNull<T>,
}

impl<T: ?Sized> Owned<T> {
    /// Creates a new `Owned` instance from a raw pointer.
    pub const unsafe fn new(val: *mut T) -> Self {
        Owned {
            val: NonNull::new(val).expect("Owned::new called with null pointer"),
        }
    }

    /// Converts the `Owned` instance into a raw pointer.
    #[must_use = "Returned value must be used to avoid memory leaks"]
    pub const unsafe fn into_raw(self) -> *mut T {
        let ptr = self.val.as_ptr();
        core::mem::forget(self); // Prevents the destructor from being called
        ptr
    }
}

impl<T: ?Sized> Deref for Owned<T> {
    type Target = T;

    /// Dereferences the `Owned` pointer to access the underlying value.
    fn deref(&self) -> &Self::Target {
        unsafe { self.val.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for Owned<T> {
    /// Mutably dereferences the `Owned` pointer to access the underlying value.
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.val.as_mut() }
    }
}

impl<T: ?Sized> Drop for Owned<T> {
    /// Drops the `Owned` instance, deallocating the memory if necessary.
    fn drop(&mut self) {
        unsafe {
            drop_in_place(self.val.as_mut());
        }
    }
}

impl<T> fmt::Debug for Owned<T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Owned({:?})", unsafe { self.val.as_ref() })
    }
}

unsafe impl<T> Send for Owned<T> where T: Send {}
unsafe impl<T> Sync for Owned<T> where T: Sync {}
