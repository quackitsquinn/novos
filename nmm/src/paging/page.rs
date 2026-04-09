use crate::{align, arch::VirtAddr, paging::PrimitiveSize};

/// A page on the current architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page<S: PrimitiveSize> {
    start_address: VirtAddr,
    _size_marker: core::marker::PhantomData<S>,
}

impl<S: PrimitiveSize> crate::NmmSealed for Page<S> {}
impl<S: PrimitiveSize> crate::paging::MemoryPrimitive<S> for Page<S> {}

impl<S: PrimitiveSize> Page<S> {
    /// Creates a new `Page` from the given starting virtual address. The address must be aligned to the size of the page, otherwise this function will panic.
    pub fn try_new(start_address: VirtAddr) -> Option<Self> {
        if align!(down, start_address.as_u64(), S::SIZE) == start_address.as_u64() {
            Some(unsafe { Self::new_unchecked(start_address) })
        } else {
            None
        }
    }

    /// Creates a new `Page` from the given starting virtual address without checking for alignment.
    ///
    /// # Safety
    /// The caller must ensure that the `start_address` is aligned to the size of the page.
    pub unsafe fn new_unchecked(start_address: VirtAddr) -> Self {
        Self {
            start_address,
            _size_marker: core::marker::PhantomData,
        }
    }

    /// Creates a new `Page` from the given starting virtual address.
    pub fn from_start_address(start_address: VirtAddr) -> Option<Self> {
        Self::try_new(start_address)
    }

    /// Creates a new `Page` that contains the given virtual address. The starting address of the page will be the largest aligned address that is less than or equal to the given address.
    pub fn containing_address(addr: VirtAddr) -> Option<Self> {
        unsafe { Self::new_unchecked(VirtAddr::new(align!(down, addr.as_u64(), S::SIZE))) }
            .try_into()
            .ok()
    }

    /// Returns the starting virtual address of the page.
    pub fn start_address(&self) -> VirtAddr {
        self.start_address
    }
}
