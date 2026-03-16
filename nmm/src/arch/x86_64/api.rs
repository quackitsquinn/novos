use cake::limine::memory_map;

use crate::{
    MapFlags, MemError,
    arch::{PhysAddr, VirtAddr},
    paging::PageTableIndex,
};

pub(crate) unsafe fn init_unchecked(
    _root: *mut (),
    _offset: VirtAddr,
    _ranges: &'static [memory_map::Entry],
    _scratch_range: (VirtAddr, u64),
) -> Result<(), MemError> {
    todo!()
}

pub(crate) unsafe fn init_load_recursive(
    _root: *mut (),
    _index: PageTableIndex,
    _phys_addr: PhysAddr,
) -> Result<(), MemError> {
    todo!("todo")
}

pub(crate) unsafe fn map_unchecked(
    _virt_base: VirtAddr,
    _phys_base: PhysAddr,
    _byte_size: u64,
    _flags: MapFlags,
) -> Result<(), MemError> {
    todo!()
}

pub(crate) unsafe fn unmap_unchecked(_virt_base: VirtAddr, _byte_size: u64) -> Result<(), MemError> {
    todo!()
}

/// Maps `byte_size` bytes of memory, returning the base virtual address of the mapped region. The physical memory for this mapping is allocated by the memory manager, and the mapping is created with the specified flags.
#[must_use = "The returned virtual address must be freed with `unmap` when it is no longer needed to avoid memory leaks and ensure proper resource management."]
pub(crate) unsafe fn alloc_paged(_byte_size: u64, _flags: MapFlags) -> Result<VirtAddr, MemError> {
    todo!()
}
