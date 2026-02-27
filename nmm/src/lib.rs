//! nmm - Novos Memory Manager Library
#![cfg_attr(not(test), no_std)]

use bitflags::bitflags;
use cake::limine::{self, memory_map};

use crate::arch::{PhysAddr, VirtAddr};

#[cfg(not(feature = "x86_64"))]
compile_error!("Only x86_64 architecture is currently supported.");

pub mod arch;
pub mod paging;

// henceforth start the trait object of "i want to define an api for the memory management and traits are easier to sketch apis out with"
trait Mm {
    /// Initializes the memory manager.
    ///
    /// - `offset` is the virtual address offset where the physical memory is mapped in the virtual address space.
    /// - `ranges` is a slice of memory map entries provided by the bootloader, describing the physical memory layout and available memory regions.
    ///    Currently, this is hardcoded to use the limine memory map, but in the future this could be converted to a more generic format to allow for different bootloaders
    ///    or custom memory map formats.
    unsafe fn init(offset: VirtAddr, ranges: &'static [memory_map::Entry]);

    /// Enables recursive paging at the specified page table index, loading the given physical address into the architecture-specific register for the page table base address.
    /// This allows the entire page table hierarchy to be accessed through a single virtual address, simplifying memory management and page table manipulation.
    ///
    /// # Safety
    /// The caller must ensure that the recursive mapping is a. present and b. pointing towards `phys_addr` before enabling it,
    /// as enabling a recursive mapping that is not properly set up can lead to undefined behavior when accessing the page tables through the recursive mapping.
    unsafe fn load_recursive(index: paging::PageTableIndex, phys_addr: PhysAddr);

    /// Maps a virtual address range to a physical address range with the specified size
    ///
    /// - `virt_base` is the starting virtual address of the range to be mapped.
    /// - `phys_base` is the starting physical address of the range to be mapped.
    /// - `byte_size` is the size of the range to be mapped, in bytes.
    /// - `flags` are the mapping flags that specify the permissions and attributes of the mapping (e.g., read/write permissions, caching behavior).
    // I thought about making this unsafe since it can cause undefined behavior BUT
    // said undefined behavior requires dereferencing the mapped memory, so the safety is the caller's responsibility.
    fn map(
        virt_base: VirtAddr,
        phys_base: PhysAddr,
        byte_size: usize,
        flags: MapFlags,
    ) -> Result<(), MapError>;
    /// Unmaps a virtual address range of the specified size starting from the given virtual base address
    ///
    /// - `virt_base` is the starting virtual address of the range to be unmapped.
    /// - `byte_size` is the size of the range to be unmapped, in bytes.
    ///
    /// # Safety
    /// This function is unsafe because unmapping memory that is still in use (e.g., memory that is currently mapped and being accessed) can lead to undefined behavior.
    /// The caller must ensure that unmapped memory is completely unused and will not be accessed after being unmapped to avoid issues such as use-after-free or memory corruption.
    unsafe fn unmap(virt_base: VirtAddr, byte_size: usize);
    /// Maps `byte_size` bytes of memory, returning the base virtual address of the mapped region. The physical memory for this mapping is allocated by the memory manager, and the mapping is created with the specified flags.
    #[must_use = "The returned virtual address must be freed with `unmap` when it is no longer needed to avoid memory leaks and ensure proper resource management."]
    fn alloc_paged(byte_size: usize, flags: MapFlags) -> Result<VirtAddr, MapError>;
}

/// An error that can occur during memory mapping operations, such as invalid addresses, insufficient resources, or permission issues. The specific variants of this error type can be defined based on the needs of the memory manager implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MapError {
    /// The mapping operation failed because the provided range was already mapped to a different physical address. The provided physical address is included for reference.
    #[error(
        "The provided virtual address range is already mapped to a different physical address: {0:?}"
    )]
    AlreadyMapped(PhysAddr),
    ///The mapping operation failed because the system ran out of memory to complete the mapping. This could occur if there are no free physical pages available to back the mapping, or if the memory manager's internal data structures are exhausted.
    #[error("The mapping operation failed due to insufficient memory resources.")]
    OutOfMemory,
}

bitflags! {
    /// Flags for memory mapping, such as read/write permissions and caching behavior.
    pub struct MapFlags: u64 {
        // PRESENT and it's forms are implicit by the usage of any map/alloc function
        /// Page is writable (if not set, the page is read-only)
        const WRITABLE = 1 << 1;
        /// Page can be accessed from user mode (if not set, the page is only accessible from kernel mode)
        const USER_ACCESSIBLE = 1 << 2;
        /// Marks the page as executable - this is only relevant on platforms with support
        /// for executable page permissions (e.g., x86_64 with the NX bit)
        const EXECUTABLE = 1 << 3;
        /// Disable caching for this page
        const CACHE_DISABLE = 1 << 4;
        /// Page is globally mapped and will exist in any address space
        const GLOBAL = 1 << 5;
    }
}
