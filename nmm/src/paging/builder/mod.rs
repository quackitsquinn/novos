//! Utility traits and types for building new address spaces. There currently is no implementation that uses these traits, however.
use crate::{
    MapFlags, MemError,
    paging::{PhysAddr, VirtAddr},
};

mod recursive;

/// The `AddressSpaceBuilder` trait defines the interface for building a new address space, such as when creating a new page table hierarchy for a process.
///  It provides a method to map virtual addresses to physical memory, with various options for how the mapping should be performed (e.g., identity mapping, copying from the current address space, allocating new memory, etc.).
/// This trait can be implemented by both the physical and virtual memory managers to manage their respective address spaces.
pub trait AddressSpaceBuilder {
    /// Maps `size` bytes of memory starting at the virtual address `virt` to physical memory according to the specified `source`. The mapping should be page-aligned and can have various options for how the physical memory is allocated or copied.
    fn map(
        &mut self,
        virt: VirtAddr,
        source: Source,
        size: u64,
        map_flags: MapFlags,
    ) -> Result<(), MemError>;
}

pub(crate) trait RemapInto {
    /// Remaps the current address space into another address space using the provided `AddressSpaceBuilder`.
    /// This is used when switching to a new address space, which is done during startup by switching from a HHDM-mapped address space to a recursive-mapped address space.
    fn remap_into<A>(&mut self, builder: &mut A) -> Result<(), MemError>
    where
        A: AddressSpaceBuilder;
}

/// The `Source` enum defines the various options for how physical memory should be allocated or copied when mapping virtual addresses in the `AddressSpaceBuilder`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Maps the virtual address to the same physical address (identity mapping).
    Identity,
    /// Maps the virtual address to the same physical address as the current address space, effectively copying the mapping from the current address space to the new one. This is useful for mapping ELF sections that are already mapped in the current address space.
    CopyFromCurrent(VirtAddr),
    /// Maps a new range of memory, without copying any data from the source address.
    Allocate {
        /// Should the memory be zeroed?
        should_zero: bool,
    },
    /// Like `CopyFromCurrent`, but with an additional `size` parameter that limits the number of bytes copied from the source address. T
    /// Bytes beyond the specified size will not be copied, and will be mapped to zeroed pages. This is useful for mapping ELF sections.
    CopyFromCurrentLimited {
        /// The source address to copy from in the current address space.
        source: VirtAddr,
        /// The number of bytes to copy from the source address. The remaining bytes in the mapped range will be zeroed.
        size: u64,
    },
    /// Maps the virtual address to a specific physical address.
    ///
    /// .0 must be aligned to at least `arch::L1_PAGE_SIZE` bytes, and the caller must ensure that the physical address is valid and not already in use.
    PhysAddr(PhysAddr),
}
