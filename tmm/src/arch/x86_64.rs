//! Architecture-specific types and implementations for x86_64.

use core::range::Range;

use bitflags::bitflags;

use crate::{
    MapFlags,
    paging::{Address, Frame, MemoryFragment, PageTableIndex, Small, VirtAddr},
};

/// The width of virtual addresses in bits for x86_64 architecture.
pub const VIRTUAL_ADDRESS_WIDTH: u8 = 48;
/// The maximum valid virtual address for x86_64 architecture.
pub const VIRTUAL_ADDRESS_MAX: u64 = (1 << VIRTUAL_ADDRESS_WIDTH) - 1;
/// The start of the higher half in virtual address space for x86_64 architecture.
pub const HIGHER_HALF_START: VirtAddr = VirtAddr::new(0xFFFF800000000000);
/// The width of physical addresses in bits for x86_64 architecture.
pub const PHYSICAL_ADDRESS_WIDTH: u8 = 52;
/// The maximum valid physical address for x86_64 architecture.
pub const PHYSICAL_ADDRESS_MAX: u64 = (1 << PHYSICAL_ADDRESS_WIDTH) - 1;
/// The number of bits used for indexing into page tables at each level.
pub const TABLE_INDEX_BITS: usize = 9;
/// The number of entries in a page table for x86_64 architecture.
pub const ENTRY_COUNT: usize = L1_PAGE_SIZE as usize / core::mem::size_of::<u64>();
/// A page table entry for x86_64 architecture, represented as a 64-bit value.
pub type PageEntryType = u64;
/// The size of a level 1 page (4KB) for x86_64 architecture.
pub const L1_PAGE_SIZE: u64 = 4096;
/// The size of a level 2 page (2MB) for x86_64 architecture.
pub const L2_PAGE_SIZE: u64 = L1_PAGE_SIZE * ENTRY_COUNT as u64;
/// The size of a level 3 page (1GB) for x86_64 architecture.
pub const L3_PAGE_SIZE: u64 = L2_PAGE_SIZE * ENTRY_COUNT as u64;

/// Page table entry flags for x86_64 architecture. This is a bitflags struct that represents the various flags that can be set in a page table entry for x86_64 architecture.
pub type ArchEntryFlags = PageTableFlags;

bitflags! {
    /// Page table entry flags for x86_64 architecture.
    #[repr(transparent)]
    pub struct PageTableFlags: u64 {
        /// The page is present in memory.
        const PRESENT         = 1 << 0;
        /// The page is writable.
        const WRITABLE        = 1 << 1;
        /// The page is accessible from userspace.
        const USER_ACCESSIBLE = 1 << 2;
        /// Write-through caching enabled.
        const WRITE_THROUGH   = 1 << 3;
        /// Cache disabled for this page.
        const NO_CACHE       = 1 << 4;
        /// The page has been accessed.
        const ACCESSED        = 1 << 5;
        /// The page has been written to.
        const DIRTY           = 1 << 6;
        /// This is a huge page (2MB or 1GB).
        const HUGE_PAGE       = 1 << 7;
        /// The page is global and not flushed from TLB on CR3 reload.
        const GLOBAL          = 1 << 8;
        /// No-execute flag; if set, code execution is not allowed from this page.
        const NO_EXECUTE      = 1 << 63;
    }
}

/// An error that originate from architecture-specific operations in the memory manager. This is the error type for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ArchError {
    #[error("Parent page table entry is a huge page, cannot map to it")]
    /// An error indicating that a mapping operation failed because the parent page table entry is a huge page, which cannot be used for mapping.
    ParentEntryHugePage,
}

pub(crate) unsafe fn do_flush(addr: VirtAddr) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "x86_64")] {
            unsafe {
                core::arch::asm!("invlpg [{}]", in(reg) addr.as_u64(), options(nostack, preserves_flags))
            };
        } else {
            let _ = addr; // Avoid unused variable warning on unsupported architectures.
        }
    }
}

pub(crate) unsafe fn do_flush_all() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "x86_64")] {
            unsafe {
                // Reload the CR3 register to flush the entire TLB.
                core::arch::asm!(
                    "mov rax, cr3; mov cr3, rax",
                    options(nostack, preserves_flags)
                )
            };
        } else {
            // No-op on unsupported architectures.
        }
    }
}

pub(crate) const fn canonicalize_phys(addr: u64) -> u64 {
    // taken from the x86_64 crate
    addr % (1 << 52)
}

pub(crate) const fn canonicalize_virt(addr: u64) -> u64 {
    // taken from the x86_64 crate

    // By doing the right shift as a signed operation (on a i64), it will
    // sign extend the value, repeating the leftmost bit.
    ((addr << 16) as i64 >> 16) as u64
}
/// Returns the physical address of the current PML4 table.
/// This function reads the. CR3 register to obtain the physical address of the PML4 table.

pub fn pml4_phys() -> Frame<Small> {
    #[cfg(target_arch = "x86_64")]
    {
        use crate::paging::PhysAddr;

        let cr3: u64;
        unsafe {
            core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
        }
        Frame::from_start_address(PhysAddr::new(canonicalize_phys(cr3))).unwrap()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        panic!("pml4_phys is only supported on x86_64 architecture");
    }
}

pub(crate) unsafe fn set_root_table(frame: Frame<Small>) {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            core::arch::asm! {
                "mov cr3, {0}",
                in(reg) frame.start_address().as_u64(),
                options(nostack, preserves_flags)
            }
        };
    }
}
/// The first free available-to-software bit in a page table entry.
pub const PTE_FREE_BIT0: u64 = 1 << 9;

/// The first slot in the pml4 table that is reserved for recursive mapping, specifically reserved for mapping of the current address space.
pub const RECURSIVE_SLOTS: Range<PageTableIndex> = Range {
    start: PageTableIndex::new(500),
    end: PageTableIndex::new(511),
};

/// The bits used for the address portion of a page table entry.
pub const PAGE_TABLE_ENTRY_ADDR_BITS: u64 = 0x000fffff_fffff000;

impl From<MapFlags> for PageTableFlags {
    fn from(value: MapFlags) -> Self {
        let mut flags = Self::PRESENT;
        if value.contains(MapFlags::WRITABLE) {
            flags |= Self::WRITABLE;
        }
        if value.contains(MapFlags::USER_ACCESSIBLE) {
            flags |= Self::USER_ACCESSIBLE;
        }
        if !value.contains(MapFlags::EXECUTABLE) {
            flags |= Self::NO_EXECUTE;
        }
        if value.contains(MapFlags::CACHE_DISABLE) {
            flags |= Self::NO_CACHE;
        }
        if value.contains(MapFlags::DEALLOCATE) {
            flags = flags | Self::from_bits_retain(PTE_FREE_BIT0);
        }
        if value.contains(MapFlags::GLOBAL) {
            flags |= Self::GLOBAL;
        }
        flags
    }
}

impl From<PageTableFlags> for MapFlags {
    fn from(value: PageTableFlags) -> Self {
        let mut flags = MapFlags::empty();
        if value.contains(PageTableFlags::WRITABLE) {
            flags |= MapFlags::WRITABLE;
        }
        if value.contains(PageTableFlags::USER_ACCESSIBLE) {
            flags |= MapFlags::USER_ACCESSIBLE;
        }
        if !value.contains(PageTableFlags::NO_EXECUTE) {
            flags |= MapFlags::EXECUTABLE;
        }
        if value.contains(PageTableFlags::NO_CACHE) {
            flags |= MapFlags::CACHE_DISABLE;
        }

        if value.bits() & PTE_FREE_BIT0 != 0 {
            flags |= MapFlags::DEALLOCATE;
        }

        if value.contains(PageTableFlags::GLOBAL) {
            flags |= MapFlags::GLOBAL;
        }
        flags
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn assert_flags_correctness() {
        use super::PageTableFlags as OurFlags;
        use x86_64::structures::paging::page_table::PageTableFlags as X86Flags;
        macro_rules! check {
            ($name: ident) => {
                assert_eq!(OurFlags::$name.bits(), X86Flags::$name.bits(), "Flag {} does not match!", stringify!($name));
            };
            (($($name: ident),+)) => {
                $(
                    check!($name);
                )+
            }
        }

        check!((
            PRESENT,
            WRITABLE,
            USER_ACCESSIBLE,
            WRITE_THROUGH,
            NO_CACHE,
            ACCESSED,
            DIRTY,
            HUGE_PAGE,
            GLOBAL,
            NO_EXECUTE
        ));
    }
}
