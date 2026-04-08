//! Architecture-specific types and implementations for the memory manager.
#[cfg(feature = "x86_64")]
pub mod x86_64;
use cake::limine::memory_map;
#[cfg(feature = "x86_64")]
use x86_64 as arch_impl;

use crate::{
    MapFlags, MemError, VirtualMemoryRange,
    entry_walker::EntryWalker,
    paging::{self},
};

/// Physical address type for the current architecture.
/// Currently, this is an alias for `arch::PhysAddr`.
pub type PhysAddr = arch_impl::PhysAddr;
/// Virtual address type for the current architecture.
/// Currently, this is an alias for `arch::VirtAddr`.
pub type VirtAddr = arch_impl::VirtAddr;
/// Page table entry type for the current architecture.
/// Currently, this is an alias for `arch::PageEntryType`.
pub type PageEntryType = arch_impl::PageEntryType;
/// An error that originate from architecture-specific operations in the memory manager. This is a wrapper around the architecture-specific error type,
/// allowing for a unified error type across the memory manager while still preserving the ability to include architecture-specific error information when necessary.
pub type ArchError = arch_impl::ArchError;

/// The start of the higher half in virtual address space.
pub const HIGHER_HALF_START: VirtAddr = VirtAddr::HIGHER_HALF_START;
/// The width of virtual addresses in bits for x86_64 architecture.
pub const VIRTUAL_ADDRESS_WIDTH: u8 = arch_impl::VIRTUAL_ADDRESS_WIDTH;
/// The maximum valid virtual address for x86_64 architecture.
pub const VIRTUAL_ADDRESS_MAX: u64 = arch_impl::VIRTUAL_ADDRESS_MAX;
/// The width of physical addresses in bits for x86_64 architecture.
pub const PHYSICAL_ADDRESS_WIDTH: u8 = arch_impl::PHYSICAL_ADDRESS_WIDTH;
/// The maximum valid physical address for x86_64 architecture.
pub const PHYSICAL_ADDRESS_MAX: u64 = arch_impl::PHYSICAL_ADDRESS_MAX;
/// The number of bits used for indexing into page tables at each level.
pub const TABLE_INDEX_BITS: usize = arch_impl::TABLE_INDEX_BITS;
/// The size of a page table in bytes for x86_64 architecture.
pub const TABLE_SIZE: u64 = arch_impl::TABLE_SIZE as u64;
/// The number of entries in a page table for x86_64 architecture.
pub const ENTRY_COUNT: usize = arch_impl::ENTRY_COUNT;

// TODO: maybe support x86 in the future? would be cool to watch this run on a xp or 98 era machine

// TODO: This functions will probably have more shared code between architectures than the public API,
// so the functions shouldn't just instantly dip into the architecture-specific implementations,
// but should have some shared code for common functionality, and then call into the architecture-specific implementations for the parts that are different between architectures. This will allow for more code reuse and less duplication between architectures, while still allowing for the necessary differences in implementation.

#[inline(always)]
pub(crate) unsafe fn init_unchecked(
    root: *mut (),
    offset: VirtAddr,
    ranges: EntryWalker<'static>,
    scratch_range: VirtualMemoryRange,
) -> Result<(), MemError> {
    unsafe { arch_impl::api::init_unchecked(root, offset, ranges, scratch_range) }
}

#[inline(always)]
pub(crate) unsafe fn init_load_recursive(
    root: *mut (),
    index: paging::PageTableIndex,
    phys_addr: PhysAddr,
) -> Result<(), MemError> {
    unsafe { arch_impl::api::init_load_recursive(root, index, phys_addr) }
}

#[inline(always)]
pub(crate) unsafe fn map_unchecked(
    virt_base: VirtAddr,
    phys_base: PhysAddr,
    byte_size: usize,
    flags: MapFlags,
) -> Result<(), MemError> {
    unsafe { arch_impl::api::map_unchecked(virt_base, phys_base, byte_size, flags) }
}

#[inline(always)]
pub(crate) unsafe fn unmap_unchecked(
    virt_base: VirtAddr,
    byte_size: usize,
) -> Result<(), MemError> {
    unsafe { arch_impl::api::unmap_unchecked(virt_base, byte_size) }
}

/// Maps `byte_size` bytes of memory, returning the base virtual address of the mapped region. The physical memory for this mapping is allocated by the memory manager, and the mapping is created with the specified flags.
#[inline(always)]
#[must_use = "The returned virtual address must be freed with `unmap` when it is no longer needed to avoid memory leaks and ensure proper resource management."]
pub(crate) unsafe fn alloc_paged(byte_size: usize, flags: MapFlags) -> Result<VirtAddr, MemError> {
    unsafe { arch_impl::api::alloc_paged(byte_size, flags) }
}
