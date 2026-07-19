//! Architecture-specific types and implementations for the memory manager.
#[cfg(feature = "x86_64")]
pub mod x86_64;
#[cfg(feature = "x86_64")]
use x86_64 as arch_impl;

use crate::{
    MapFlags, MemError,
    entry_walker::EntryWalker,
    paging::{self, PageTable, PhysAddr, VirtAddr},
};

/// Page table entry type for the current architecture.
/// Currently, this is an alias for `arch::PageEntryType`.
pub type PageEntryType = arch_impl::PageEntryType;
/// An error that originate from architecture-specific operations in the memory manager. This is a wrapper around the architecture-specific error type,
/// allowing for a unified error type across the memory manager while still preserving the ability to include architecture-specific error information when necessary.
pub type ArchError = arch_impl::ArchError;
/// Page table flags type for the current architecture.
/// This needs a Impl and From implementation to convert from the architecture-agnostic `MapFlags` to the architecture-specific flags used in page table entries.
pub type ArchEntryFlags = arch_impl::PageTableFlags;
/// The API for architecture-specific operations in the memory manager.
/// This is a wrapper around the architecture-specific API, allowing for a unified interface for architecture-specific operations while still preserving the
/// ability to include architecture-specific implementations when necessary.
pub type Mapper = arch_impl::Mapper;

/// The start of the higher half in virtual address space.
pub const HIGHER_HALF_START: VirtAddr = arch_impl::HIGHER_HALF_START;
/// The width of virtual addresses in bits for the current architecture.
pub const VIRTUAL_ADDRESS_WIDTH: u8 = arch_impl::VIRTUAL_ADDRESS_WIDTH;
/// The maximum valid virtual address for the current architecture.
pub const VIRTUAL_ADDRESS_MAX: u64 = arch_impl::VIRTUAL_ADDRESS_MAX;
/// The width of physical addresses in bits for the current architecture.
pub const PHYSICAL_ADDRESS_WIDTH: u8 = arch_impl::PHYSICAL_ADDRESS_WIDTH;
/// The maximum valid physical address for the current architecture.
pub const PHYSICAL_ADDRESS_MAX: u64 = arch_impl::PHYSICAL_ADDRESS_MAX;
/// The number of bits used for indexing into page tables at each level.
pub const TABLE_INDEX_BITS: usize = arch_impl::TABLE_INDEX_BITS;
/// The number of entries in a page table for the current architecture.
pub const ENTRY_COUNT: usize = arch_impl::ENTRY_COUNT;
/// The size of a level 1 page (4KB) for the current architecture.
pub const L1_PAGE_SIZE: u64 = arch_impl::L1_PAGE_SIZE;
/// The size of a level 2 page (2MB) for the current architecture.
pub const L2_PAGE_SIZE: u64 = arch_impl::L2_PAGE_SIZE;
/// The size of a level 3 page (1GB) for the current architecture.
pub const L3_PAGE_SIZE: u64 = arch_impl::L3_PAGE_SIZE;

// TODO: maybe support x86 in the future? would be cool to watch this run on a xp or 98 era machine

pub(crate) use arch_impl::api::init_load_recursive;
pub(crate) use arch_impl::api::init_unchecked;

pub(crate) use arch_impl::do_flush;
pub(crate) use arch_impl::do_flush_all;

pub(crate) use arch_impl::canonicalize_phys;
pub(crate) use arch_impl::canonicalize_virt;

pub(crate) use arch_impl::PTE_FREE_BIT0;

/// Validates that the given physical address is valid for the current architecture.
pub const fn is_valid_phys(addr: u64) -> bool {
    canonicalize_phys(addr) == addr
}

/// Validates that the given virtual address is valid for the current architecture.
pub const fn is_valid_virt(addr: u64) -> bool {
    canonicalize_virt(addr) == addr
}
