//! This module defines the `Page` struct, which represents a virtual memory page of a specific size (small, medium, or large) on the current architecture.
//! It also defines the `UnsizedPage` enum, which can represent a page of any size.
use crate::paging::primitives::{AnyFragment, PageClass, Primitive};
use crate::paging::{Address, Large, Medium, Small, VirtAddr};
use crate::{align, paging::FragmentSize};
use core::any::type_name;
use core::fmt::Debug;
use core::mem::transmute;

/// A page on the current architecture.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Page<S: FragmentSize> {
    start_address: VirtAddr,
    _size_marker: core::marker::PhantomData<S>,
}

impl<S: FragmentSize> crate::NmmSealed for Page<S> {}
impl<S: FragmentSize> Primitive for Page<S> {}

impl<S: FragmentSize> const crate::paging::MemoryFragment<S> for Page<S> {
    type AddressType = VirtAddr;

    fn start_address(&self) -> VirtAddr {
        self.start_address
    }

    fn containing_address(addr: Self::AddressType) -> Self {
        unsafe { Self::new_unchecked(VirtAddr::new_truncate(align!(down, addr.as_u64(), S::SIZE))) }
    }

    fn from_start_address(start_address: Self::AddressType) -> Option<Self> {
        Self::try_new(start_address)
    }
}

impl<S: FragmentSize> Page<S> {
    /// Attempts to create a new `Page` from the given starting virtual address. The address must be aligned to the size of the page, otherwise this function will return `None`.
    pub const fn try_new_u64(start_address: u64) -> Option<Self> {
        Self::try_new(VirtAddr::new(start_address))
    }

    /// Creates a new `Page` from the given starting virtual address. The address must be aligned to the size of the page, otherwise this function will panic.
    pub const fn try_new(start_address: VirtAddr) -> Option<Self> {
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
    pub const unsafe fn new_unchecked(start_address: VirtAddr) -> Self {
        Self {
            start_address,
            _size_marker: core::marker::PhantomData,
        }
    }

    /// Creates a new `Page` from the given starting virtual address.
    pub const fn from_start_address(start_address: VirtAddr) -> Option<Self> {
        Self::try_new(start_address)
    }

    /// Returns the starting virtual address of the page.
    pub const fn start_address(&self) -> VirtAddr {
        self.start_address
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
