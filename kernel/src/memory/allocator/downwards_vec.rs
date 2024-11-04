use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// A downwards-growing vector. Used for the block table in the allocator.
pub struct DownwardsVec<'a, T> {
    base: *mut T,
    len: usize,
    capacity: usize,
    slice: &'a mut [T],
    _marker: PhantomData<T>,
}

impl<'a, T> DownwardsVec<'a, T> {
    /// Create a new downwards-growing vector.
    /// # Safety
    /// The caller must ensure that the base pointer is valid for the entire lifetime of the vector. The caller must also ensure that base - (capacity * size_of::<T>()) is a valid pointer.
    pub unsafe fn new(base: *mut T, capacity: usize) -> Self {
        Self {
            base,
            len: 0,
            capacity,
            slice: unsafe { core::slice::from_raw_parts_mut(base, 0) },
            _marker: PhantomData,
        }
    }

    fn check_capacity(&self, additional: usize) -> bool {
        self.len + additional <= self.capacity
    }

    pub unsafe fn push_unchecked(&mut self, value: T) {
        unsafe {
            // Write the value to the base pointer, then increment the base pointer.
            if self.len != 0 {
                self.base = self.base.sub(1);
            }
            self.len += 1;
            core::ptr::write(self.base, value);
            self.slice = core::slice::from_raw_parts_mut(self.base, self.len);
        }
    }

    pub fn push(&mut self, value: T) -> Option<()> {
        if self.check_capacity(1) {
            // Safety: We just checked that there is enough capacity
            unsafe { self.push_unchecked(value) };
            Some(())
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn as_ptr(&self) -> *const T {
        self.base
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.base
    }

    pub unsafe fn set_cap(&mut self, cap: usize) {
        assert!(
            cap >= self.len,
            "New capacity must be greater than or equal to the length"
        );
        self.capacity = cap;
    }

    pub unsafe fn set_len(&mut self, len: usize) {
        assert!(
            len <= self.capacity,
            "New length must be less than or equal to the capacity"
        );
        self.len = len;
        self.slice = unsafe { core::slice::from_raw_parts_mut(self.base, self.len) };
    }

    pub unsafe fn grow(&mut self, additional: usize) {
        self.capacity += additional;
    }
    /// Clone the vector exactly. This will create a new vector with the same length, capacity, and elements.
    /// # Safety
    /// This is *incredibly* unsafe. The caller must ensure the following:
    /// - The cloned vector is forgotten before the original vector is dropped.
    /// - The cloned vector is not used after the original vector is dropped.
    /// - There are no concurrent accesses to either the original or cloned vector.
    ///
    /// This function is primarily used for testing to create a test vector that can be cloned into a test allocator.
    pub unsafe fn clone_exact(&self) -> Self {
        Self {
            base: self.base,
            len: self.len,
            capacity: self.capacity,
            slice: unsafe { core::slice::from_raw_parts_mut(self.base, self.len) },
            _marker: PhantomData,
        }
    }
}

impl<'a, T> core::ops::Deref for DownwardsVec<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.slice
    }
}

impl<'a, T> core::ops::DerefMut for DownwardsVec<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slice
    }
}

impl<'a, T> Drop for DownwardsVec<'a, T> {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.len {
                core::ptr::drop_in_place(self.base.add(i));
            }
        }
    }
}

impl<'a, T> Index<usize> for DownwardsVec<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.slice[index]
    }
}

impl<'a, T> IndexMut<usize> for DownwardsVec<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.slice[index]
    }
}

impl<'a, T> core::fmt::Debug for DownwardsVec<'a, T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.slice.fmt(f)
    }
}

#[cfg(test)]
pub mod test {
    use core::mem::MaybeUninit;

    

    

    use super::*;
    /// Create a new downwards-growing vector in the given array.
    pub fn new_in<'a, T, const CAP: usize>(
        here: &'a mut [MaybeUninit<T>; CAP],
    ) -> DownwardsVec<'a, T> {
        let base = unsafe { (here.as_mut_ptr() as *mut T).add(CAP - 1) };
        unsafe { DownwardsVec::new(base, CAP) }
    }

    #[kproc::test("DownwardsVec push one", can_recover = true)]
    fn test_push_one() {
        let mut arr = [MaybeUninit::uninit(); 10];
        let mut vec = new_in(&mut arr);
        vec.push(1).unwrap();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0], 1);
    }

    #[kproc::test("DownwardsVec push fill", can_recover = true)]
    fn test_push_fill() {
        let mut arr = [MaybeUninit::uninit(); 10];
        let mut vec = new_in(&mut arr);
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        assert_eq!(vec.len(), 10);
        for i in 0..10 {
            assert_eq!(vec[i], 9 - i);
        }
    }

    #[kproc::test("DownwardsVec push over cap", can_recover = true)]
    fn test_push_over_cap() {
        let mut arr = [MaybeUninit::uninit(); 10];
        let mut vec = new_in(&mut arr);
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        assert_eq!(vec.len(), 10);
        assert!(vec.push(10).is_none());
        assert_eq!(vec.len(), 10);
    }

    #[kproc::test("DownwardsVec clone exact", can_recover = true)]
    fn test_clone_exact() {
        let mut arr = [MaybeUninit::uninit(); 10];
        let mut vec = new_in(&mut arr);
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        let mut clone = unsafe { vec.clone_exact() };
        assert!(clone.slice == vec.slice);
        assert!(clone.base == vec.base);
        assert!(clone.len == vec.len);
        assert!(clone.capacity == vec.capacity);
    }

    #[kproc::test("DownwardsVec iter")]
    fn test_iter() {
        let mut arr = [MaybeUninit::uninit(); 10];
        let mut vec = new_in(&mut arr);
        for i in 0..10 {
            vec.push(i).unwrap();
        }
        let mut iter = vec.iter();
        for i in 0..10 {
            assert_eq!(iter.next(), Some(&(9 - i)));
        }
        assert_eq!(iter.next(), None);
    }
}
