//! Contains the core types and structures related to paging, such as page table entries, page tables, and the layout of the page table hierarchy. It also defines the virtual and physical address types used by the architecture.
pub mod index;

pub use index::PageTableIndex;

use crate::arch::PageEntryType;

/// The virtual address type used by the current architecture.
pub type VirtAddr = crate::arch::VirtAddr;
/// The physical address type used by the current architecture.
pub type PhysAddr = crate::arch::PhysAddr;

/// The type used for page table entries in the current architecture.
pub type Table = [PageEntryType; crate::arch::ENTRY_COUNT];

/// The layout of the a page table structure in memory. This is used to determine how to calculate the addresses of page tables and entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureLayout {
    /// A recursive layout, where the page tables are arranged in a recursive manner.
    ///
    /// This requires one entry in the highest level page table to point back to itself, allowing the entire page table hierarchy to be accessed through a single virtual address.
    Recursive(PageTableIndex),
    /// A direct mapping layout, where the virtual address of a page table entry is directly derived from its physical address.
    /// This is common in higher half kernels where the entire physical memory is mapped at a fixed offset in the virtual address space.
    DirectMapping(VirtAddr),
}
