use cake::limine::memory_map;

use crate::{
    MapFlags, MemError,
    arch::{PhysAddr, VirtAddr},
    paging::PageTableIndex,
};

pub(crate) unsafe fn init_unchecked(
    offset: VirtAddr,
    ranges: &'static [memory_map::Entry],
    scratch_range: (VirtAddr, u64),
) -> Result<(), MemError> {
    todo!("todo")
}

pub(crate) unsafe fn init_load_recursive(
    index: PageTableIndex,
    phys_addr: PhysAddr,
) -> Result<(), MemError> {
    todo!("todo")
}

pub(crate) unsafe fn map_unchecked(
    virt_base: VirtAddr,
    phys_base: PhysAddr,
    byte_size: u64,
    flags: MapFlags,
) -> Result<(), MemError> {
    todo!()
}

pub(crate) unsafe fn unmap_unchecked(virt_base: VirtAddr, byte_size: u64) -> Result<(), MemError> {
    todo!()
}

/// Maps `byte_size` bytes of memory, returning the base virtual address of the mapped region. The physical memory for this mapping is allocated by the memory manager, and the mapping is created with the specified flags.
#[must_use = "The returned virtual address must be freed with `unmap` when it is no longer needed to avoid memory leaks and ensure proper resource management."]
pub(crate) unsafe fn alloc_paged(byte_size: u64, flags: MapFlags) -> Result<VirtAddr, MemError> {
    todo!()
}
