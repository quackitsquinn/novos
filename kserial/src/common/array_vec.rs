use core::{
    fmt::Debug,
    ops::{Deref, Index, IndexMut},
};

use bytemuck::{Pod, Zeroable};

/// A vector with a fixed capacity that is stored on the stack.
///
/// This implementation of `ArrayVec` has a very specific memory layout, requiring a unique implementation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrayVec<T, const CAP: usize>
where
    T: Pod,
{
    len: u16,
    data: [T; CAP],
}

impl<T: Pod, const CAP: usize> ArrayVec<T, CAP> {
    /// Creates a new `ArrayVec` with a length of 0.
    pub fn new() -> Self {
        ArrayVec {
            len: 0,
            data: [Zeroable::zeroed(); CAP],
        }
    }

    /// Creates a new `ArrayVec` from a length and an array of `MaybeUninit`.
    pub unsafe fn from_raw_parts(len: u16, data: [T; CAP]) -> Self {
        ArrayVec { len, data }
    }

    /// Returns the length of the vector.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns the pointer to the data.
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr() as *const T
    }
}

impl<const CAP: usize> ArrayVec<u8, CAP> {
    /// Create an `ArrayVec` from a string slice.
    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() > CAP {
            return None;
        }

        let mut data: [u8; CAP] = [0; CAP];
        let mut len = 0;

        for c in s.chars() {
            data[len] = c as u8; // Convert the character to a byte
            len += 1;
        }

        Some(ArrayVec {
            len: len as u16,
            data,
        })
    }
    /// Try to convert the `ArrayVec` to a `String`.
    #[cfg(feature = "std")]
    pub fn try_to_string(&self) -> Option<String> {
        let mut s = String::with_capacity(self.len());
        for i in 0..self.len() {
            if self[i] as char == '\n' {}
            s.push(self[i] as char);
        }
        Some(s)
    }
}

impl<T, const CAP: usize> ArrayVec<T, CAP>
where
    T: Pod,
{
    /// Create an `ArrayVec` from raw bytes without validation.
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        let mut data: [T; CAP] = [Zeroable::zeroed(); CAP];
        let mut len = 0;

        for chunk in bytes.chunks_exact(core::mem::size_of::<T>()) {
            let chunk = bytemuck::from_bytes(chunk);
            data[len] = *chunk;
            len += 1;
        }

        ArrayVec {
            len: len as u16,
            data,
        }
    }

    /// Try to create an `ArrayVec` from raw bytes with validation.
    pub fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > CAP * core::mem::size_of::<T>() {
            return None;
        }
        if bytes.len() % core::mem::size_of::<T>() != 0 {
            return None;
        }

        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }

    /// Create an `ArrayVec` from raw bytes, panicking on failure.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() > CAP * core::mem::size_of::<T>() {
            panic!("ArrayVec::from_bytes: bytes too long");
        }
        if bytes.len() % core::mem::size_of::<T>() != 0 {
            panic!("ArrayVec::from_bytes: bytes not aligned");
        }

        unsafe { Self::from_bytes_unchecked(bytes) }
    }
}

impl<T: Pod, const CAP: usize> Index<usize> for ArrayVec<T, CAP> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.len() {
            panic!("ArrayVec::index: index out of bounds");
        }

        &self.data[index]
    }
}

impl<T: Pod, const CAP: usize> IndexMut<usize> for ArrayVec<T, CAP> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.len() {
            panic!("ArrayVec::index_mut: index out of bounds");
        }

        &mut self.data[index]
    }
}

unsafe impl<T: Pod, const CAP: usize> Zeroable for ArrayVec<T, CAP> {}
unsafe impl<T: Pod, const CAP: usize> Pod for ArrayVec<T, CAP> {}

impl<T: Pod + PartialEq, const CAP: usize> PartialEq for ArrayVec<T, CAP> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        for i in 0..self.len() {
            if self[i] != other[i] {
                return false;
            }
        }

        true
    }
}

impl<T: Pod + Eq, const CAP: usize> Eq for ArrayVec<T, CAP> {}

impl<T: Pod, const CAP: usize> Deref for ArrayVec<T, CAP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len()) }
    }
}

impl<T: Pod + Debug, const CAP: usize> Debug for ArrayVec<T, CAP> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Validate + Pod, const CAP: usize> Validate for ArrayVec<T, CAP> {
    fn validate(&self) -> bool {
        // First off, validate ourselves
        if self.len() > CAP {
            return false;
        }
        // Then validate each element
        for i in 0..self.len() {
            if !self[i].validate() {
                return false;
            }
        }
        true
    }
}

mod arr_vec_macro {
    macro_rules! varlen {
        ($ty: ty, $cap: expr) => {
            $crate::common::array_vec::ArrayVec<$ty, {$cap}>
        };
    }
    pub(crate) use varlen;
}

pub(crate) use arr_vec_macro::varlen;

use super::validate::Validate;
