//! This module defines the `Page` struct, which represents a virtual memory page of a specific size (small, medium, or large) on the current architecture.
//! It also defines the `UnsizedPage` enum, which can represent a page of any size.
use crate::paging::primitives::Primitive;
use crate::paging::{Address, Large, Medium, Small, VirtAddr};
use crate::{align, paging::PrimitiveSize};
use core::any::type_name;
use core::fmt::Debug;

/// A page on the current architecture.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Page<S: PrimitiveSize> {
    start_address: VirtAddr,
    _size_marker: core::marker::PhantomData<S>,
}

impl<S: PrimitiveSize> crate::NmmSealed for Page<S> {}
impl<S: PrimitiveSize> Primitive for Page<S> {}

impl<S: PrimitiveSize> const crate::paging::MemoryFragment<S> for Page<S> {
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

impl<S: PrimitiveSize> Page<S> {
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

impl<S: PrimitiveSize> Debug for Page<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Page")
            .field(&type_name::<S>())
            .field(&self.start_address)
            .finish()
    }
}
/// An enum representing a virtual memory page of any size (small, medium, or large).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsizedPage {
    /// A small page, typically 4KB in size for x86_64 architecture.
    Small(Page<Small>),
    /// A medium page, typically 2MB in size for x86_64 architecture.
    Medium(Page<Medium>),
    /// A large page, typically 1GB in size for x86_64 architecture.
    Large(Page<Large>),
}

impl<S> Into<UnsizedPage> for Page<S>
where
    S: PrimitiveSize,
{
    fn into(self) -> UnsizedPage {
        // SAFETY: Page<S> is guaranteed to be valid for itself.
        match S::SIZE {
            Small::SIZE => UnsizedPage::Small(unsafe { Page::new_unchecked(self.start_address) }),
            Medium::SIZE => UnsizedPage::Medium(unsafe { Page::new_unchecked(self.start_address) }),
            Large::SIZE => UnsizedPage::Large(unsafe { Page::new_unchecked(self.start_address) }),
            _ => panic!("Invalid page size"),
        }
    }
}
