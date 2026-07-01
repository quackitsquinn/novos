//! This module defines the `Frame` struct, which represents a physical memory frame of a specific size (small, medium, or large) on the current architecture.
//! It also defines the `UnsizedFrame` enum, which can represent a frame of any size.
use core::{any::type_name, fmt::Debug, mem::transmute};

use crate::{
    NmmSealed, align,
    paging::{
        Address, FragmentSize, Large, Medium, MemoryFragment, PhysAddr, Small,
        primitives::{AnyFragment, FrameClass, Primitive},
    },
};

/// A physical memory frame on the current architecture.
/// A frame represents a contiguous block of physical memory that can be mapped into the virtual address space with a page of the same size.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Frame<S: FragmentSize> {
    start_address: PhysAddr,
    _size_marker: core::marker::PhantomData<S>,
}

impl<S: FragmentSize> Frame<S> {
    /// Creates a new `Frame` from the given starting physical address. The address must be aligned to the size of the frame, otherwise this function will panic.
    pub const fn new(start_address: PhysAddr) -> Self {
        Self::try_new(start_address)
            .expect("Frame::new: start_address is not aligned to frame size")
    }

    /// Tries to create a new `Frame` from the given starting physical address. Returns `None` if the address is not aligned to the size of the frame.
    pub const fn try_new(start_address: PhysAddr) -> Option<Self> {
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
    pub const unsafe fn new_unchecked(start_address: PhysAddr) -> Self {
        Self {
            start_address,
            _size_marker: core::marker::PhantomData,
        }
    }

    /// Creates a new `Frame` from the given starting physical address.
    pub const fn from_start_address(start_address: PhysAddr) -> Option<Self> {
        Self::try_new(start_address)
    }

    /// Returns the starting physical address of the frame.
    pub const fn start_address(&self) -> PhysAddr {
        self.start_address
    }
}

impl<S: FragmentSize> NmmSealed for Frame<S> {}
impl<S: FragmentSize> Primitive for Frame<S> {}

impl<S: FragmentSize> const MemoryFragment<S> for Frame<S> {
    type AddressType = PhysAddr;

    fn from_start_address(start_address: Self::AddressType) -> Option<Self> {
        Self::try_new(start_address)
    }

    fn containing_address(addr: Self::AddressType) -> Self {
        unsafe { Self::new_unchecked(PhysAddr::new(align!(down, addr.as_u64(), S::SIZE))) }
    }

    /// Returns the starting physical address of the frame.
    fn start_address(&self) -> PhysAddr {
        self.start_address
    }
}

impl<S: FragmentSize> Debug for Frame<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Frame<{}> {{ start_address: {:#x} }}",
            S::NAME,
            self.start_address.as_u64()
        )
    }
}

impl<S: FragmentSize> From<Frame<S>> for AnyFragment<FrameClass> {
    fn from(frame: Frame<S>) -> Self {
        // SAFETY: We know that Fragment<Small> == Frame<Small> and so on, so we can safely transmute between them.
        match S::SIZE {
            Small::SIZE => AnyFragment::Small(unsafe { transmute(frame) }),
            Medium::SIZE => AnyFragment::Medium(unsafe { transmute(frame) }),
            Large::SIZE => AnyFragment::Large(unsafe { transmute(frame) }),
            _ => unreachable!("Invalid frame size"),
        }
    }
}
