//! Address primitives for x86_64 architecture.

use core::ops::Sub;
use core::ops::{Deref, DerefMut};

use crate::paging::Table;
use crate::{arch::x86_64::VIRTUAL_ADDRESS_WIDTH, paging::StructureLayout};

// You might wonder why we don't just re-export x86_64::PhysAddr and x86_64::VirtAddr directly.
// There are a few reasons for this:
// 1. I don't have a concrete design for how to handle multiple architectures yet.
//    By wrapping these types, I can more easily swap out implementations later.
// 2. If I want to add a method to PhysAddr or VirtAddr, I can't do that directly on the types from the x86_64 crate.
//    Wrapping them allows me to add methods as needed.

/// The pointer-sized unsigned integer type for the current architecture. This is used for addresses and offsets.
pub type AddressType = u64;

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

    /// Adds the given offset to this virtual address, returning `None` if the result would overflow or be out of bounds for the architecture.
    pub const fn add_checked(&self, offset: u64) -> Option<Self> {
        // First, just add it, and if it overflows, return None.
        // We can't short circuit this check, since this is a const function.
        let res = match self.0.as_u64().checked_add(offset) {
            Some(val) => val,
            None => return None,
        };

        // Then, check if the result is a valid virtual address for the architecture. If not, return None.
        if res > super::VIRTUAL_ADDRESS_MAX {
            None
        } else {
            Some(PhysAddr(x86_64::PhysAddr::new_truncate(res)))
        }
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
    // TODO: Move these constants and some of the associated functions to a implementation in ::arch
    /// The bit width of virtual addresses on x86_64.
    pub const BIT_WIDTH: u8 = VIRTUAL_ADDRESS_WIDTH; // Note: 52 bit virtual addresses are *possible*, but it's so new that we'll stick with 48 for now.
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

    /// Converts this virtual address to a physical address with a bitwise identical representation.
    pub const fn as_phys_addr(&self) -> PhysAddr {
        PhysAddr(x86_64::PhysAddr::new_truncate(self.as_u64()))
    }

    /// Adds the given offset to this virtual address, returning `None` if the result would overflow or be out of bounds for the architecture.
    pub const fn add_checked(&self, offset: u64) -> Option<Self> {
        // First, just add it, and if it overflows, return None.
        // We can't short circuit this check, since this is a const function.
        let res = match self.0.as_u64().checked_add(offset) {
            Some(val) => val,
            None => return None,
        };

        // Then, check if the result is a valid virtual address for the architecture. If not, return None.
        if res > super::VIRTUAL_ADDRESS_MAX {
            None
        } else {
            Some(VirtAddr(x86_64::VirtAddr::new_truncate(res)))
        }
    }
}

impl Deref for VirtAddr {
    type Target = x86_64::VirtAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::PhysAddr;

    #[test]
    fn test_phys_addr_add_checked() {
        let addr = PhysAddr::new(0x1000);
        assert_eq!(addr.add_checked(0x1000), Some(PhysAddr::new(0x2000)));
        assert_eq!(addr.add_checked(0xFFFFFFFFFFFFF000), None); // This would overflow
        assert_eq!(addr.add_checked(0xFFFFFFFFFFFFE000), None); // This would be out of bounds
    }

    #[test]
    fn test_virt_addr_add_checked() {
        let addr = super::VirtAddr::new(0x1000);
        assert_eq!(addr.add_checked(0x1000), Some(super::VirtAddr::new(0x2000)));
        assert_eq!(addr.add_checked(0xFFFFFFFFFFFFF000), None); // This would overflow
        assert_eq!(addr.add_checked(0xFFFFFFFFFFFFE000), None); // This would be out of bounds
    }
}
