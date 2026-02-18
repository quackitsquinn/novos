//! Address primitives for x86_64 architecture.

use core::ops::{Deref, DerefMut};

// You might wonder why we don't just re-export x86_64::PhysAddr and x86_64::VirtAddr directly.
// There are a few reasons for this:
// 1. I don't have a concrete design for how to handle multiple architectures yet.
//    By wrapping these types, I can more easily swap out implementations later.
// 2. If I want to add a method to PhysAddr or VirtAddr, I can't do that directly on the types from the x86_64 crate.
//    Wrapping them allows me to add methods as needed.

/// Physical address type for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PhysAddr(x86_64::PhysAddr);

impl PhysAddr {
    /// The bit width of physical addresses on x86_64.
    pub const BIT_WIDTH: u8 = 52;

    /// Create a new physical address from a u64.
    pub const fn new(addr: u64) -> Self {
        PhysAddr(x86_64::PhysAddr::new(addr))
    }

    /// Get the underlying u64 value of the physical address.
    pub const fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }
}

impl Deref for PhysAddr {
    type Target = x86_64::PhysAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PhysAddr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Virtual address type for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtAddr(x86_64::VirtAddr);

impl VirtAddr {
    /// The bit width of virtual addresses on x86_64.
    pub const BIT_WIDTH: u8 = 48; // Note: 52 bit virtual addresses are *possible*, but it's so new that we'll stick with 48 for now.
    /// The start of the higher half in virtual address space.
    pub const HIGHER_HALF_START: VirtAddr =
        VirtAddr(x86_64::VirtAddr::new_truncate(1 << (Self::BIT_WIDTH - 1)));

    /// Create a new virtual address from a u64.
    pub const fn new(addr: u64) -> Self {
        VirtAddr(x86_64::VirtAddr::new(addr))
    }

    /// Create a new virtual address from a u64, truncating any bits beyond the architecture's bit width. This will sign-extend the address if necessary.
    pub const fn new_truncate(addr: u64) -> Self {
        VirtAddr(x86_64::VirtAddr::new_truncate(addr))
    }

    /// Get the underlying u64 value of the virtual address.
    pub const fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }

    /// Get the underlying usize value of the virtual address.
    /// The returned usize will always be canonical, since this is only active when the architecture's pointer width is 64 bits.
    pub const fn as_usize(&self) -> usize {
        self.0.as_u64() as usize
    }
}

impl Deref for VirtAddr {
    type Target = x86_64::VirtAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
