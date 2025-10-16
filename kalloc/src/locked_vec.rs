//! A manually managed vector that requires explicit capacity management.
use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr,
};

/// A mostly raw vector that requires explicit capacity management.
///
/// This type is mainly intended for use in kernel allocators, where the vector cannot use rustc's inbuilt allocation mechanisms.
pub struct LockedVec<T> {
    base: *mut T,
    len: usize,
    capacity: usize,
    _marker: PhantomData<T>,
}

impl<T> LockedVec<T> {
    /// Creates a new locked allocation vector with the given base pointer and capacity.
    /// # Safety
    /// The caller must ensure the following:
    /// - The base pointer is valid for reads and writes for `capacity` elements.
    /// - The base pointer is not used for any other purpose while the vector is alive.
    pub unsafe fn new(base: *mut T, capacity: usize) -> Self {
        assert!(base.is_aligned(), "base pointer must be aligned");
        unsafe {
            base.write_bytes(0, capacity);
        }
        Self {
            base,
            len: 0,
            capacity,
            _marker: PhantomData,
        }
    }

    /// Returns the length of the vector.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Returns the capacity of the vector.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    /// Returns the base pointer of the vector.
    pub fn base(&self) -> *mut T {
        self.base
    }

    /// Pushes a new element to the vector. This function does not check for capacity.
    pub fn push_unchecked(&mut self, value: T) {
        unsafe {
            self.base.add(self.len).write(value);
        }
        self.len += 1;
    }

    /// Pushes a new element to the vector. This function checks for capacity.
    /// Returns `None` if the vector is full.
    pub fn push(&mut self, value: T) -> Option<()> {
        if self.len == self.capacity {
            return None;
        }
        self.push_unchecked(value);
        Some(())
    }
    /// Pops an element from the vector. Returns `None` if the vector is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        unsafe { Some(self.base.add(self.len).read()) }
    }
    /// Clears the vector.
    pub fn clear(&mut self) {
        // SAFETY: The vector is the sole owner of the memory.
        unsafe {
            core::ptr::drop_in_place(core::slice::from_raw_parts_mut(self.base, self.len));
        }
        self.len = 0;
    }

    /// Removes an element from the vector at the given index.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }
        let value = unsafe { ptr::read(self.base.add(index)) };
        unsafe {
            ptr::copy(
                self.base.add(index + 1),
                self.base.add(index),
                self.len - index - 1,
            );
        }
        self.len -= 1;
        Some(value)
    }

    /// Grows the vector downwards.
    /// # Safety
    /// The caller must ensure that the new pointer is valid.
    pub unsafe fn grow_down(&mut self, count: usize) {
        let old_ptr = self.base;
        let new_ptr = unsafe { self.base.sub(count) };
        self.capacity += count;
        self.base = new_ptr;
        // SAFETY: The caller must ensure that the new pointer is valid.
        unsafe { ptr::copy(old_ptr, new_ptr, self.len) };
    }
    /// Returns whether the vector is at capacity.
    pub fn at_capacity(&self) -> bool {
        self.len == self.capacity
    }

    /// Returns the byte size of the vector.
    pub fn byte_size(&self) -> usize {
        self.capacity * core::mem::size_of::<T>()
    }
}

impl<T> Deref for LockedVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.base, self.len) }
    }
}

impl<T> DerefMut for LockedVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.base, self.len) }
    }
}

impl<T> Index<usize> for LockedVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.deref()[index]
    }
}

impl<T> IndexMut<usize> for LockedVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.deref_mut()[index]
    }
}

impl<T> Debug for LockedVec<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.deref()).finish()
    }
}

impl<T> Drop for LockedVec<T> {
    fn drop(&mut self) {
        // SAFETY: The vector is the sole owner of the memory.
        unsafe {
            for i in 0..self.len {
                ptr::drop_in_place(self.base.add(i));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::{alloc::Layout, slice};

    // As a notice, the usage of `_defer_guard` is INCREDIBLY IMPORTANT and can cause tests to SPONTANEOUSLY FAIL if not used.
    // This is because binding it to `_` will cause the guard to be dropped immediately, which will deallocate the memory before the test is run.
    // This is kinda dumb, and not made clear in the places I looked and was hell to debug.

    use crate::{locked_vec::LockedVec, test_common::DeferDealloc};

    fn new_vec<T>(cap: usize) -> (LockedVec<T>, DeferDealloc) {
        let (dropper, ptr) = DeferDealloc::alloc(Layout::array::<T>(cap).expect("layout"));
        let vec = unsafe { LockedVec::new(ptr.as_ptr() as *mut T, cap) };
        (vec, dropper)
    }

    #[test]
    fn test_push() {
        for _ in 0..30 {
            let (mut vec, keeper) = new_vec::<u32>(4);
            assert!(vec.push(1).is_some());
            assert!(vec.push(2).is_some());
            assert!(vec.push(3).is_some());
            assert!(vec.push(4).is_some());
            assert!(vec.push(5).is_none());

            assert_eq!(vec.len(), 4);
            assert_eq!(&*vec, &[1, 2, 3, 4]);
            drop(keeper);
        }
    }

    #[test]
    fn test_pop() {
        for _ in 0..30 {
            let (mut vec, keeper) = new_vec::<u32>(4);
            assert!(vec.push(1).is_some());
            assert!(vec.push(2).is_some());
            assert!(vec.push(3).is_some());
            assert!(vec.push(4).is_some());

            assert_eq!(vec.pop(), Some(4));
            assert_eq!(vec.pop(), Some(3));
            assert_eq!(vec.pop(), Some(2));
            assert_eq!(vec.pop(), Some(1));
            assert_eq!(vec.pop(), None);
            drop(keeper);
        }
    }

    #[test]
    fn test_remove() {
        for _ in 0..30 {
            let (mut vec, keeper) = new_vec::<u32>(4);
            assert!(vec.push(1).is_some());
            assert!(vec.push(2).is_some());
            assert!(vec.push(3).is_some());
            assert!(vec.push(4).is_some());

            assert_eq!(vec.remove(1), Some(2));
            assert_eq!(vec.remove(2), Some(4));
            assert_eq!(vec.remove(0), Some(1));
            assert_eq!(vec.remove(0), Some(3));
            assert_eq!(vec.remove(0), None);
            drop(keeper);
        }
    }

    #[test]
    fn test_clear() {
        let (mut vec, _defer_guard) = new_vec::<u32>(4);
        assert!(vec.push(1).is_some());
        assert!(vec.push(2).is_some());
        assert!(vec.push(3).is_some());
        assert!(vec.push(4).is_some());

        vec.clear();

        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 4);
    }

    #[test]
    fn test_byte_size() {
        let (vec, _defer_guard) = new_vec::<u32>(4);
        assert_eq!(vec.byte_size(), 16);
    }
}
