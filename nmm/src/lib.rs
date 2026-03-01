//! nmm - Novos Memory Manager Library
#![cfg_attr(not(test), no_std)]

use bitflags::bitflags;
use cake::limine::{self, memory_map};

use crate::arch::{PhysAddr, VirtAddr};

#[cfg(not(feature = "x86_64"))]
compile_error!("Only x86_64 architecture is currently supported.");

pub mod arch;
pub mod paging;

/// Initializes the memory manager.
///
/// - `offset` is the virtual address offset where the physical memory is mapped in the virtual address space.
/// - `ranges` is a slice of memory map entries provided by the bootloader, describing the physical memory layout and available memory regions.
///    Currently, this is hardcoded to use the limine memory map, but in the future this could be converted to a more generic format to allow for different bootloaders
///    or custom memory map formats.
/// - `scratch_range` is a tuple containing a virtual address and size that's used as a backing for virtual memory operations during initialization, and for `alloc_paged`
pub unsafe fn init(
    offset: VirtAddr,
    ranges: &'static [memory_map::Entry],
    scratch_range: (VirtAddr, u64),
) -> Result<(), MemError> {
    check_range_virt(scratch_range.0, scratch_range.1)?;
    unsafe { arch::init_unchecked(offset, ranges, scratch_range) }
}

/// Enables recursive paging at the specified page table index, loading the given physical address into the architecture-specific register for the page table base address.
/// This allows the entire page table hierarchy to be accessed through a single virtual address, simplifying memory management and page table manipulation.
///
/// # Safety
/// The caller must ensure that the recursive mapping is a. present and b. pointing towards `phys_addr` before enabling it,
/// as enabling a recursive mapping that is not properly set up can lead to undefined behavior when accessing the page tables through the recursive mapping.
pub unsafe fn load_recursive(
    index: paging::PageTableIndex,
    phys_addr: PhysAddr,
) -> Result<(), MemError> {
    if (phys_addr.as_u64() & arch::TABLE_SIZE as u64 - 1) != 0 {
        return Err(MemError::InvalidVirtRange {
            begin: VirtAddr::new(phys_addr.as_u64()),
            size: arch::TABLE_SIZE as u64,
        });
    }
    unsafe { arch::init_load_recursive(index, phys_addr) }
}

/// Maps a virtual address range to a physical address range with the specified size
///
/// - `virt_base` is the starting virtual address of the range to be mapped.
/// - `phys_base` is the starting physical address of the range to be mapped.
/// - `byte_size` is the size of the range to be mapped, in bytes.
/// - `flags` are the mapping flags that specify the permissions and attributes of the mapping (e.g., read/write permissions, caching behavior).
// I thought about making this unsafe since it can cause undefined behavior BUT
// said undefined behavior requires dereferencing the mapped memory, so the safety is the caller's responsibility.
pub fn map(
    virt_base: VirtAddr,
    phys_base: PhysAddr,
    byte_size: u64,
    flags: MapFlags,
) -> Result<(), MemError> {
    check_range_virt(virt_base, byte_size)?;
    check_range_phys(phys_base, byte_size)?;
    unsafe { arch::map_unchecked(virt_base, phys_base, byte_size, flags) }
}

/// Unmaps a virtual address range of the specified size starting from the given virtual base address
///
/// - `virt_base` is the starting virtual address of the range to be unmapped.
/// - `byte_size` is the size of the range to be unmapped, in bytes.
///
/// # Safety
/// This function is unsafe because unmapping memory that is still in use (e.g., memory that is currently mapped and being accessed) can lead to undefined behavior.
/// The caller must ensure that unmapped memory is completely unused and will not be accessed after being unmapped to avoid issues such as use-after-free or memory corruption.
pub unsafe fn unmap(virt_base: VirtAddr, byte_size: u64) -> Result<(), MemError> {
    check_range_virt(virt_base, byte_size as u64)?;
    unsafe { arch::unmap_unchecked(virt_base, byte_size) }
}

/// Maps `byte_size` bytes of memory, returning the base virtual address of the mapped region. The physical memory for this mapping is allocated by the memory manager, and the mapping is created with the specified flags.
///
/// `byte_size` will always be rounded up to the nearest page size, so the actual mapped size may be larger than the requested size.
#[must_use = "The returned virtual address must be freed with `unmap` when it is no longer needed to avoid memory leaks and ensure proper resource management."]
pub fn alloc_paged(byte_size: u64, flags: MapFlags) -> Result<VirtAddr, MemError> {
    if byte_size > arch::VIRTUAL_ADDRESS_MAX {}
    unsafe { arch::alloc_paged(byte_size, flags) }
}

/// An error that can occur during memory mapping operations, such as invalid addresses, insufficient resources, or permission issues. The specific variants of this error type can be defined based on the needs of the memory manager implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MemError {
    /// The mapping operation failed because the provided range was already mapped to a different physical address. The provided physical address is included for reference.
    #[error(
        "The provided virtual address range is already mapped to a different physical address: {0:?}"
    )]
    AlreadyMapped(PhysAddr),
    /// The mapping operation failed because the system ran out of memory to complete the mapping.
    /// This could occur if there are no free physical pages available to back the mapping, or if the memory manager's internal data structures are exhausted.
    #[error("The mapping operation failed due to insufficient memory resources.")]
    OutOfMemory,
    /// The provided virtual address range is invalid, such as being out of bounds for the architecture or overflowing when calculating the end address.
    #[error("The provided virtual address range is invalid: {begin:?} with size {size} bytes")]
    InvalidVirtRange {
        /// The starting virtual address of the invalid range.
        begin: VirtAddr,
        /// The size of the invalid range in bytes.
        size: u64,
    },
    /// The provided physical address range is invalid, such as being out of bounds for the architecture or overflowing when calculating the end address.
    #[error("The provided physical address range is invalid: {begin:?} with size {size} bytes")]
    InvalidPhysRange {
        /// The starting physical address of the invalid range.
        begin: PhysAddr,
        /// The size of the invalid range in bytes.
        size: u64,
    },
    /// An error that originated from architecture-specific operations in the memory manager.
    #[error("An architecture-specific error occurred during memory management operations: {0}")]
    ArchError(#[from] arch::ArchError),
}

fn check_range_virt(virt_base: VirtAddr, byte_size: u64) -> Result<(), MemError> {
    // Check for overflow in the address range
    virt_base
        .add_checked(byte_size)
        .ok_or(MemError::InvalidVirtRange {
            begin: virt_base,
            size: byte_size,
        })
        .map(|_| ())
}

fn check_range_phys(phys_base: PhysAddr, byte_size: u64) -> Result<(), MemError> {
    // Check for overflow in the address range
    phys_base
        .add_checked(byte_size)
        .ok_or(MemError::InvalidPhysRange {
            begin: phys_base,
            size: byte_size,
        })
        .map(|_| ())
}

bitflags! {
    /// Flags for memory mapping, such as read/write permissions and caching behavior.
    pub struct MapFlags: u64 {
        // PRESENT and it's forms are implicit by the usage of any map/alloc function
        // GLOBAL is similar, any allocation or mapping is implicitly global if it's in kernel space.
        /// Page is writable (if not set, the page is read-only)
        const WRITABLE = 1 << 1;
        /// Page can be accessed from user mode (if not set, the page is only accessible from kernel mode)
        const USER_ACCESSIBLE = 1 << 2;
        /// Marks the page as executable - this is only relevant on platforms with support
        /// for executable page permissions (e.g., x86_64 with the NX bit)
        const EXECUTABLE = 1 << 3;
        /// Disable caching for this page
        const CACHE_DISABLE = 1 << 4;
    }
}
