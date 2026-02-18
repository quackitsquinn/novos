pub mod index;

pub use index::PageTableIndex;

/// The virtual address type used by the current architecture.
pub type VirtAddr = crate::arch::VirtAddr;
/// The physical address type used by the current architecture.
pub type PhysAddr = crate::arch::PhysAddr;

/// The layout of the a page table structure in memory. This is used to determine how to calculate the addresses of page tables and entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureLayout {
    /// A recursive layout, where the page tables are arranged in a recursive manner. This is common in x86_64 where the top-level page table (P4) is mapped into itself, allowing for easy access to all page tables and entries through a fixed virtual address range.
    Recursive(PageTableIndex),
    /// A direct mapping layout, where the virtual address of a page table entry is directly derived from its physical address.
    /// This is common in higher half kernels where the entire physical memory is mapped at a fixed offset in the virtual address space.
    DirectMapping(u64),
}
