use crate::{
    NmmSealed, align,
    arch::PhysAddr,
    paging::{MemoryPrimitive, PrimitiveSize},
};

/// A physical memory frame on the current architecture.
/// A frame represents a contiguous block of physical memory that can be mapped into the virtual address space with a page of the same size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<S: PrimitiveSize> {
    start_address: PhysAddr,
    _size_marker: core::marker::PhantomData<S>,
}

impl<S: PrimitiveSize> Frame<S> {
    /// Creates a new `Frame` from the given starting physical address. The address must be aligned to the size of the frame, otherwise this function will panic.
    pub fn new(start_address: PhysAddr) -> Self {
        Self::try_new(start_address)
            .expect("Frame::new: start_address is not aligned to frame size")
    }

    /// Tries to create a new `Frame` from the given starting physical address. Returns `None` if the address is not aligned to the size of the frame.
    pub fn try_new(start_address: PhysAddr) -> Option<Self> {
        if align!(down, start_address.as_u64(), S::SIZE) == start_address.as_u64() {
            Some(unsafe { Self::new_unchecked(start_address) })
        } else {
            None
        }
    }

    /// Creates a new `Frame` from the given starting physical address without checking for alignment.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the `start_address` is aligned to the size of the frame.
    pub unsafe fn new_unchecked(start_address: PhysAddr) -> Self {
        Self {
            start_address,
            _size_marker: core::marker::PhantomData,
        }
    }

    /// Creates a new `Frame` from the given starting physical address.
    pub fn from_start_address(start_address: PhysAddr) -> Option<Self> {
        Self::try_new(start_address)
    }

    /// Creates a new `Frame` that contains the given physical address. The starting address of the frame will be the largest aligned address that is less than or equal to the given address.
    pub fn containing_address(addr: PhysAddr) -> Option<Self> {
        unsafe { Self::new_unchecked(PhysAddr::new(align!(down, addr.as_u64(), S::SIZE))) }
            .try_into()
            .ok()
    }

    /// Returns the starting physical address of the frame.
    pub fn start_address(&self) -> PhysAddr {
        self.start_address
    }
}

impl<S: PrimitiveSize> NmmSealed for Frame<S> {}
impl<S: PrimitiveSize> MemoryPrimitive<S> for Frame<S> {}
