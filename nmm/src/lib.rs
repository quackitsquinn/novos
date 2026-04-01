//! nmm - Novos Memory Manager Library
#![cfg_attr(not(test), no_std)]
#![feature(ptr_alignment_type)]
#![feature(trace_macros)]

use bitflags::bitflags;
use cake::limine::memory_map;

#[doc(hidden)]
pub use pastey as _pastey;

use crate::{
    arch::{PhysAddr, VirtAddr},
    bitmap::GLOBAL_BITMAP,
    entry_walker::EntryWalker,
};

#[cfg(not(feature = "x86_64"))]
compile_error!("Only x86_64 architecture is currently supported.");

pub mod arch;
pub mod bitmap;
pub mod entry_walker;
pub mod kernel_map;
pub mod paging;

/// A range of virtual memory, guaranteed to be valid for the architecture (e.g., canonical for x86_64) and properly aligned to page boundaries. This is used for managing virtual address space and ensuring that allocated virtual addresses are valid and usable for mapping physical memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualMemoryRange {
    pub(crate) base: VirtAddr,
    pub(crate) size: u64,
}

impl VirtualMemoryRange {
    /// Creates a new `VirtualMemoryRange` with the given base address and size, validating that the range is valid for the architecture.
    /// This includes checks for overflow, alignment, and canonical form (if applicable).
    pub const fn new(base: VirtAddr, size: u64) -> Self {
        match check_range_virt(base, size) {
            Ok(()) => (),
            Err(e) => match e {
                MemError::InvalidVirtRange { reason, .. } => match reason {
                    InvalidVirtRangeReason::Overflow => panic!("Virtual memory range overflow"),
                    InvalidVirtRangeReason::NonCanonical => {
                        panic!("Virtual memory range is not canonical")
                    }
                    InvalidVirtRangeReason::Unaligned => {
                        panic!("Virtual memory range is not aligned to page size:")
                    }
                },
                _ => unreachable!(), // check_range_virt can only return InvalidVirtRange errors
            },
        }
        Self { base, size }
    }

    /// Returns the base virtual address of the range.
    pub fn base(&self) -> VirtAddr {
        self.base
    }

    /// Returns the size of the virtual memory range in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }
}

/// Initializes the memory manager.
///
/// - `root` is a pointer to the root page table structure, which will be used as the base for all page table operations. The specific type and
///          structure of this root page table is architecture-specific and will be defined in the architecture-specific implementation.
///          This allows the memory manager to work with different page table formats and structures depending on the architecture being used.
/// - `offset` is the virtual address offset where the physical memory is mapped in the virtual address space.
/// - `ranges` is a slice of memory map entries provided by the bootloader, describing the physical memory layout and available memory regions.
///    Currently, this is hardcoded to use the limine memory map, but in the future this could be converted to a more generic format to allow for different bootloaders
///    or custom memory map formats.
/// - `managed_range` is a tuple containing the starting virtual address and size of a range of virtual memory manager will manage.
///   This range is used for virtual address allocation (e.g., for `alloc_virtspace`) and physical memory mapping (e.g., for `alloc_paged`),
///   as well as internal memory management state.
pub unsafe fn init(
    root: *mut (),
    offset: VirtAddr,
    ranges: &'static [&'static memory_map::Entry],
    managed_range: VirtualMemoryRange,
) -> Result<(), MemError> {
    assert!(
        !root.is_null(),
        "The root page table pointer must not be null."
    );

    unsafe { arch::init_unchecked(root, offset, EntryWalker::new(ranges), managed_range) }
}

/// Enables recursive paging at the specified page table index, loading the given physical address into the architecture-specific register for the page table base address.
/// This allows the entire page table hierarchy to be accessed through a single virtual address, simplifying memory management and page table manipulation.
///
/// # Safety
/// The caller must ensure that the recursive mapping is a. present and b. pointing towards `phys_addr` before enabling it,
/// as enabling a recursive mapping that is not properly set up can lead to undefined behavior when accessing the page tables through the recursive mapping.
pub unsafe fn load_recursive(
    root: *mut (),
    index: paging::PageTableIndex,
    phys_addr: PhysAddr,
) -> Result<(), MemError> {
    if (phys_addr.as_u64() & arch::TABLE_SIZE as u64 - 1) != 0 {
        return Err(MemError::InvalidVirtRange {
            reason: InvalidVirtRangeReason::Unaligned,
            begin: VirtAddr::new(phys_addr.as_u64()),
            size: arch::TABLE_SIZE as u64,
        });
    }
    assert!(
        !root.is_null(),
        "The root page table pointer must not be null."
    );

    unsafe { arch::init_load_recursive(root, index, phys_addr) }
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

/// Allocates a virtual address range of the specified size without mapping it to any physical memory.
#[must_use = "The returned virtual address must be freed with `free_virtspace` when it is no longer needed to avoid memory leaks and ensure proper resource management."]
pub fn alloc_virtspace(byte_size: u64, alignment: u64) -> Result<VirtAddr, MemError> {
    assert!(
        alignment.is_power_of_two(),
        "Alignment must be a power of two, but got {}",
        alignment
    );

    if byte_size > arch::VIRTUAL_ADDRESS_MAX {
        return Err(MemError::OutOfMemory);
    }

    let bitmap = GLOBAL_BITMAP
        .try_get()
        .expect("Global bitmap must be initialized before allocating virtual address space");
}

/// Frees a virtual address range of the specified size that was previously allocated with `alloc_virtspace`.
///
/// # Safety
/// The caller must ensure that the provided virtual address range is not currently mapped to any physical memory and is not in use before freeing it,
/// as freeing a virtual address range that is still in use can lead to undefined behavior such as use-after-free or memory corruption.
pub unsafe fn free_virtspace(virt_base: VirtAddr, byte_size: u64) -> Result<(), MemError> {
    check_range_virt(virt_base, byte_size)?;
    todo!("bitmap allocator for virtual address space");
}

/// Maps a physical address range to a virtual address range of the specified size with the given flags, where the virtual address is allocated by the memory manager. This is a convenience function that combines `alloc_paged` and `map` into a single operation for ease of use.
#[must_use = "The returned virtual address must be freed with `unmap` when it is no longer needed to avoid memory leaks and ensure proper resource management."]
// TODO: how to handle freeing the virtspace allocated by this function?
pub fn map_alloc(
    phys_addr: PhysAddr,
    byte_size: u64,
    flags: MapFlags,
) -> Result<VirtAddr, MemError> {
    check_range_phys(phys_addr, byte_size)?;
    let virt_addr = alloc_virtspace(byte_size)?;
    unsafe { arch::map_unchecked(virt_addr, phys_addr, byte_size, flags) }?;
    Ok(virt_addr)
}

/// The reason why a virtual address range was rejected as invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidVirtRangeReason {
    /// The virtual address range would overflow when calculating the end address (i.e., `virt_base + byte_size` would overflow).
    Overflow,
    /// The virtual address range is not canonical for the architecture (e.g., on x86_64, the upper bits beyond the architecture's virtual address width are not all 0s or all 1s).
    NonCanonical,
    /// The virtual address range is not aligned to the required page size (i.e., `virt_base` is not a multiple of the page size, or `byte_size` is not a multiple of the page size).
    Unaligned,
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
        /// The specific reason why the virtual address range is considered invalid (e.g., overflow, unaligned, etc.).
        reason: InvalidVirtRangeReason,
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
    /// The provided scratch space for initialization is too small to be used for the necessary virtual memory operations during initialization and `alloc_paged`.
    #[error(
        "The provided scratch space is too small: provided {provided} bytes, but at least {required} bytes are required."
    )]
    ScratchSpaceTooSmall {
        /// The size of the provided scratch space in bytes.
        provided: u64,
        /// The minimum required size of the scratch space in bytes.
        required: u64,
    },
    #[error(
        "The proper resources to complete the requested operation has not been initialized yet. \n This can occur if the global bitmap has not been initialized before calling an operation that relies on it, such as `alloc_virtspace`."
    )]
    Uninit,
}

const fn check_range_virt(virt_base: VirtAddr, byte_size: u64) -> Result<(), MemError> {
    // Check for overflow in the address range
    let val = virt_base.checked_add(byte_size);
    match val {
        Some(_) => (),
        None => {
            return Err(MemError::InvalidVirtRange {
                reason: InvalidVirtRangeReason::Overflow,
                begin: virt_base,
                size: byte_size,
            });
        }
    };

    // Check page alignment, since `virt_base` is a VirtAddr, it's defined to be canonical.
    if (virt_base.as_u64() % arch::TABLE_SIZE as u64) != 0 {
        return Err(MemError::InvalidVirtRange {
            reason: InvalidVirtRangeReason::Unaligned,
            begin: virt_base,
            size: byte_size,
        });
    }

    Ok(())
}

fn check_range_phys(phys_base: PhysAddr, byte_size: u64) -> Result<(), MemError> {
    // Check for overflow in the address range
    phys_base
        .checked_add(byte_size)
        .ok_or(MemError::InvalidPhysRange {
            begin: phys_base,
            size: byte_size,
        })
        .map(|_| ())
}

bitflags! {
    /// Flags for memory mapping, such as read/write permissions and caching behavior.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Aligns the given value up or down to the nearest multiple of the specified alignment. The alignment must be a power of two.
///
/// # Usage
///
/// ```
/// let aligned_up = align!(up, 0x1234, 0x1000); // Aligns 0x1234 up to the nearest multiple of 0x100
/// let aligned_down = align!(down, 0x1234, 0x1000); // Aligns 0x1234 down to the nearest multiple of 0x100
///   ```
#[macro_export]
macro_rules! align {
    (up, $value: expr, $alignment: expr) => {
        (($value + $alignment - 1) / $alignment) * {
            const _: () = {
                assert!(
                    ($alignment as u64).is_power_of_two(),
                    "Alignment must be a power of two"
                )
            };
            $alignment
        }
    };

    (down, $value: expr, $alignment: expr) => {
        const _: () = {
            assert!(
                $alignment.is_power_of_two(),
                "Alignment must be a power of two"
            );
        };
        ($value / $alignment) * $alignment
    };
}
