//! Architecture-specific types and implementations for x86_64.

pub mod addr;

use bitflags::bitflags;

pub use addr::{PhysAddr, VirtAddr};

/// The width of virtual addresses in bits for x86_64 architecture.
pub const VIRTUAL_ADDRESS_WIDTH: usize = 48;
/// The width of physical addresses in bits for x86_64 architecture.
pub const PHYSICAL_ADDRESS_WIDTH: usize = 52;
/// The number of bits used for indexing into page tables at each level.
pub const TABLE_INDEX_BITS: usize = 9;
/// The size of a page table in bytes for x86_64 architecture.
pub const TABLE_SIZE: usize = 4096;
/// The number of entries in a page table for x86_64 architecture.
pub const ENTRY_COUNT: usize = TABLE_SIZE / core::mem::size_of::<u64>();

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
