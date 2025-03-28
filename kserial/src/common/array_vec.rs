use core::{
    mem::MaybeUninit,
    ops::{Index, IndexMut},
};

use bytemuck::{Pod, Zeroable};

/// A vector with a fixed capacity that is stored on the stack.
///
/// This implementation of `ArrayVec` has a very specific memory layout, requiring a unique implementation.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ArrayVec<T, const CAP: usize>
where
    T: Pod,
{
    len: u16,
    data: [MaybeUninit<T>; CAP],
}

impl<T: Pod, const CAP: usize> ArrayVec<T, CAP> {
    /// Creates a new `ArrayVec` with a length of 0.
    pub fn new() -> Self {
        ArrayVec {
            len: 0,
            data: [const { MaybeUninit::uninit() }; CAP],
        }
    }

    /// Creates a new `ArrayVec` from a length and an array of `MaybeUninit`.
    pub unsafe fn from_raw_parts(len: u16, data: [MaybeUninit<T>; CAP]) -> Self {
        ArrayVec { len, data }
    }

    /// Returns the length of the vector.
    pub fn len(&self) -> usize {
        self.len as usize
    }
}

impl<const CAP: usize> ArrayVec<u8, CAP> {
    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() > CAP {
            return None;
        }

        let mut data: [MaybeUninit<u8>; CAP] = [const { MaybeUninit::uninit() }; CAP];
        let mut len = 0;

        for c in s.chars() {
            data[len] = MaybeUninit::new(c as u8);
            len += 1;
        }

        Some(ArrayVec {
            len: len as u16,
            data,
        })
    }
    #[cfg(feature = "std")]
    pub fn try_to_string(&self) -> Option<String> {
        let mut s = String::with_capacity(self.len());
        for i in 0..self.len() {
            s.push(self[i] as char);
        }
        Some(s)
    }
}

impl<T, const CAP: usize> ArrayVec<T, CAP>
where
    T: Pod,
{
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        let mut data: [MaybeUninit<T>; CAP] = [MaybeUninit::uninit(); CAP];
        let mut len = 0;

        for chunk in bytes.chunks_exact(core::mem::size_of::<T>()) {
            let chunk = bytemuck::from_bytes(chunk);
            data[len] = MaybeUninit::new(*chunk);
            len += 1;
        }

        ArrayVec {
            len: len as u16,
            data,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > CAP * core::mem::size_of::<T>() {
            return None;
        }
        if bytes.len() % core::mem::size_of::<T>() != 0 {
            return None;
        }

        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }

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

        unsafe { &*self.data[index].as_ptr() }
    }
}

impl<T: Pod, const CAP: usize> IndexMut<usize> for ArrayVec<T, CAP> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.len() {
            panic!("ArrayVec::index_mut: index out of bounds");
        }

        unsafe { &mut *self.data[index].as_mut_ptr() }
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
