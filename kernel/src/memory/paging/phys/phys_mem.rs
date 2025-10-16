//! Physical memory maps and unmapping
use cake::log::{info, trace};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{Mapper, PageTableFlags, PhysFrame, mapper::MapToError},
};

use crate::memory::paging::{
    ACTIVE_PAGE_TABLE, KernelPageSize,
    vaddr_mapper::{VIRT_MAPPER, VirtualAddressRange},
};

/// Represents a mapping of physical memory into the virtual address space.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[must_use = "The memory map must be unmapped when it is no longer needed"]
pub struct PhysicalMemoryMap {
    phys_addr: PhysAddr,
    virt_addr: VirtAddr,
    size: u64,
    virt_range: VirtualAddressRange,
}

impl PhysicalMemoryMap {
    unsafe fn new(
        phys_addr: PhysAddr,
        virt_addr: VirtAddr,
        size: u64,
        virt_range: VirtualAddressRange,
    ) -> Self {
        Self {
            phys_addr,
            virt_addr,
            size,
            virt_range,
        }
    }

    /// Returns a raw pointer to the mapped virtual memory.
    pub fn ptr(&self) -> *const u8 {
        self.virt_addr.as_ptr()
    }

    /// Returns the size of the memory.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns the physical address of the mapped memory.
    pub fn phys_addr(&self) -> PhysAddr {
        self.phys_addr
    }
}

/// Maps a physical address range into the virtual address space.
///
/// # Safety
///
/// The caller must ensure that the physical address range is valid and not already mapped.
pub unsafe fn map_address(
    addr: PhysAddr,
    byte_size: u64,
    flags: PageTableFlags,
) -> Result<PhysicalMemoryMap, MapError> {
    let base_frame = PhysFrame::containing_address(addr);
    let end_frame = PhysFrame::containing_address(PhysAddr::new(addr.as_u64() + byte_size - 1));
    let mut range = PhysFrame::range_inclusive(base_frame, end_frame);

    // It's page aligned, so we can map it directly
    let mut vmapper = VIRT_MAPPER.get();
    let addr_range = vmapper
        .allocate(range.len())
        .ok_or(MapError::UnableToAllocateVirtualMemory)?;

    info!(
        "Mapping physical memory: {:x} - {:x} to virtual {:x} - {:x} ({} frame(s))",
        base_frame.start_address().as_u64(),
        end_frame.start_address().as_u64() + 0x1000,
        addr_range.start.as_u64(),
        addr_range.end().as_u64(),
        range.len()
    );

    let mut active_page_table = ACTIVE_PAGE_TABLE.write();
    let mut frame_allocator = crate::memory::paging::phys::FRAME_ALLOCATOR.get();
    for page in addr_range.as_page_range() {
        let frame = range
            .next()
            .expect("Mapped too many pages, ran out of frames. This is a bug.");
        trace!(
            "Mapping page {:x} to frame {:x}",
            page.start_address().as_u64(),
            frame.start_address().as_u64()
        );

        unsafe {
            active_page_table
                .map_to(page, frame, flags, &mut *frame_allocator)
                .map(|f| f.flush())
                .map_err(|_| {
                    MapError::MappingError(MapToError::PageAlreadyMapped(
                        active_page_table
                            .translate_page(page)
                            .expect("just mapped, should translate"),
                    ))
                })?;
        }
    }
    return Ok(unsafe {
        PhysicalMemoryMap::new(
            addr,
            VirtAddr::new(addr_range.as_page().start_address().as_u64() + (addr.as_u64() & 0xFFF)),
            byte_size,
            addr_range,
        )
    });
}

/// Unmaps a previously mapped physical memory range.
pub fn unmap_address(map: PhysicalMemoryMap) {
    let mut vmapper = VIRT_MAPPER.get();
    let mut active_page_table = ACTIVE_PAGE_TABLE.write();
    for page in map.virt_range.as_page_range() {
        active_page_table
            .unmap(page)
            .expect("Unable to unmap page")
            .1
            .flush();
    }
    vmapper.deallocate(map.virt_range);
}

/// Remaps a previously mapped physical memory range to a new size with specified flags.
///
/// # Safety
///
/// The caller must ensure that the physical memory is not accessed after unmapping.
#[must_use = "The memory map must be unmapped when it is no longer needed"]
pub fn remap_address(
    map: &PhysicalMemoryMap,
    new_size: u64,
    flags: PageTableFlags,
) -> Result<PhysicalMemoryMap, MapError> {
    // TODO: Optimize this to only map/unmap the difference if the new size is smaller/larger
    unmap_address(*map);
    unsafe { map_address(map.phys_addr, new_size, flags) }
}

/// Errors that can occur when mapping physical memory.
#[derive(Debug, thiserror::Error)]
pub enum MapError {
    /// Unable to allocate virtual memory
    #[error("Unable to allocate virtual memory")]
    UnableToAllocateVirtualMemory,
    /// Mapping error
    #[error("Mapping error")]
    MappingError(MapToError<KernelPageSize>),
}

mod tests {
    use kproc::test;
    use x86_64::structures::paging::FrameAllocator;

    use crate::memory::paging::phys::FRAME_ALLOCATOR;

    #[test("try map")]
    fn test_map() {
        // Get a physical frame
        let mut allocator = FRAME_ALLOCATOR.get();
        let frame = allocator.allocate_frame().unwrap();
        drop(allocator);
        // Map it
        let map = unsafe {
            super::map_address(
                frame.start_address(),
                4096,
                x86_64::structures::paging::PageTableFlags::PRESENT
                    | x86_64::structures::paging::PageTableFlags::WRITABLE,
            )
        }
        .unwrap();
        //Check that the mapping is correct
        assert_eq!(map.phys_addr(), frame.start_address());
        //Unmap it
        super::unmap_address(map);
    }
}
