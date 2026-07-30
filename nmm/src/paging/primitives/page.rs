//! This module defines the `Page` struct, which represents a virtual memory page of a specific size (small, medium, or large) on the current architecture.
//! It also defines the `UnsizedPage` enum, which can represent a page of any size.
use crate::paging::primitives::{AnyFragment, PageClass, Primitive};
use crate::paging::{Address, AddressExt, Large, Medium, Small, VirtAddr};
use crate::{align, paging::FragmentSize};
use core::fmt::Debug;
use core::mem::transmute;

/// A page on the current architecture.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page<S: FragmentSize> {
    start_address: VirtAddr,
    _size_marker: core::marker::PhantomData<S>,
}

impl<S: FragmentSize> crate::NmmSealed for Page<S> {}
impl<S: FragmentSize> Primitive for Page<S> {}

const impl<S: FragmentSize> crate::paging::MemoryFragment<S> for Page<S> {
    type AddressType = VirtAddr;

    fn start_address(&self) -> VirtAddr {
        self.start_address
    }

    fn containing_address(addr: Self::AddressType) -> Self {
        unsafe {
            Self::from_start_address_unchecked(VirtAddr::new_truncate(align!(
                down,
                addr.as_u64(),
                S::SIZE
            )))
        }
    }

    unsafe fn from_start_address_unchecked(start_address: Self::AddressType) -> Self {
        Self {
            start_address,
            _size_marker: core::marker::PhantomData,
        }
    }
}

impl<S: FragmentSize> Page<S> {
    /// Zeros the memory of the page.
    ///
    /// # Safety
    /// The page must be mapped to physical memory and must be writable.
    pub unsafe fn zero(&self) {
        let ptr = self.start_address.as_mut_ptr::<u8>();
        unsafe { core::ptr::write_bytes(ptr, 0, S::SIZE as usize) };
    }
}

impl<S: FragmentSize> Debug for Page<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Page<{}> {{ start_address: {:#x} }}",
            S::NAME,
            self.start_address.as_u64()
        )
    }
}

impl<S: FragmentSize> From<Page<S>> for AnyFragment<PageClass> {
    fn from(frame: Page<S>) -> Self {
        // SAFETY: We know that Fragment<Small> == Page<Small> and so on, so we can safely transmute between them.
        match S::SIZE {
            Small::SIZE => AnyFragment::Small(unsafe { transmute(frame) }),
            Medium::SIZE => AnyFragment::Medium(unsafe { transmute(frame) }),
            Large::SIZE => AnyFragment::Large(unsafe { transmute(frame) }),
            _ => unreachable!("Invalid frame size"),
        }
    }
}
