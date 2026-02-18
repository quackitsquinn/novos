//! Architecture-specific types and implementations for the memory manager.
#[cfg(feature = "x86_64")]
pub mod x86_64;
#[cfg(feature = "x86_64")]
use x86_64 as arch_impl;

/// Physical address type for the current architecture.
/// Currently, this is an alias for `arch::PhysAddr`.
pub type PhysAddr = arch_impl::PhysAddr;
/// Virtual address type for the current architecture.
/// Currently, this is an alias for `arch::VirtAddr`.
pub type VirtAddr = arch_impl::VirtAddr;

/// The start of the higher half in virtual address space.
pub const HIGHER_HALF_START: VirtAddr = VirtAddr::HIGHER_HALF_START;
/// The width of virtual addresses in bits for x86_64 architecture.
pub const VIRTUAL_ADDRESS_WIDTH: usize = arch_impl::VIRTUAL_ADDRESS_WIDTH;
/// The width of physical addresses in bits for x86_64 architecture.
pub const PHYSICAL_ADDRESS_WIDTH: usize = arch_impl::PHYSICAL_ADDRESS_WIDTH;
/// The number of bits used for indexing into page tables at each level.
pub const TABLE_INDEX_BITS: usize = arch_impl::TABLE_INDEX_BITS;
/// The size of a page table in bytes for x86_64 architecture.
pub const TABLE_SIZE: usize = arch_impl::TABLE_SIZE;
/// The number of entries in a page table for x86_64 architecture.
pub const ENTRY_COUNT: usize = arch_impl::ENTRY_COUNT;

// TODO: maybe support x86 in the future? would be cool to watch this run on a xp or 98 era machine
