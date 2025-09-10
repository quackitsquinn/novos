use x86_64::{
    structures::paging::{Mapper, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

use crate::memory::paging::{
    vaddr_mapper::{VirtualAddressRange, VIRT_MAPPER},
    KERNEL_PAGE_TABLE,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[must_use = "The memory map must be unmapped when it is no longer needed"]
pub struct PhysicalMemoryMap {
    phys_addr: PhysAddr,
    virt_addr: VirtAddr,
    size: u64,
    offset: u64,
    virt_range: VirtualAddressRange,
}

impl PhysicalMemoryMap {
    unsafe fn new(
        phys_addr: PhysAddr,
        virt_addr: VirtAddr,
        size: u64,
        offset: u64,
        virt_range: VirtualAddressRange,
    ) -> Self {
        debug_assert!(offset < size, "Offset must be less than size");
        Self {
            phys_addr,
            virt_addr,
            size,
            offset,
            virt_range,
        }
    }

    pub fn ptr(&self) -> *const u8 {
        (self.virt_addr.as_u64() + self.offset) as *const u8
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn phys_addr(&self) -> PhysAddr {
        self.phys_addr
    }
}

pub fn map_address(
    addr: PhysAddr,
    size: u64,
    flags: PageTableFlags,
) -> Result<PhysicalMemoryMap, MapError> {
    let base_frame = PhysFrame::containing_address(addr);
    let end_frame = PhysFrame::containing_address(PhysAddr::new(addr.as_u64() + size - 1));
    let mut range = PhysFrame::range_inclusive(base_frame, end_frame);

    // It's page aligned, so we can map it directly
    let mut vmapper = VIRT_MAPPER.get();
    let addr_range = vmapper
        .allocate(range.len())
        .ok_or(MapError::UnableToAllocateVirtualMemory)?;

    let inner_page_offset = addr.as_u64() % 4096;

    let mut offset_page_table = KERNEL_PAGE_TABLE.get();
    let mut frame_allocator = crate::memory::paging::phys::FRAME_ALLOCATOR.get();
    for (page, frame) in addr_range.as_page_range().zip(&mut range) {
        unsafe {
            offset_page_table
                .map_to(page, frame, flags, &mut *frame_allocator)
                .map(|f| f.flush())
                .map_err(|_| MapError::MappingError)?;
        }
    }
    return Ok(unsafe {
        PhysicalMemoryMap::new(
            addr,
            VirtAddr::new(addr.as_u64()),
            size,
            inner_page_offset,
            addr_range,
        )
    });
}

pub fn unmap_address(map: PhysicalMemoryMap) {
    let mut vmapper = VIRT_MAPPER.get();
    let mut offset_page_table = KERNEL_PAGE_TABLE.get();
    for page in map.virt_range.as_page_range() {
        offset_page_table
            .unmap(page)
            .expect("Unable to unmap page")
            .1
            .flush();
    }
    vmapper.deallocate(map.virt_range);
}

#[must_use = "The memory map must be unmapped when it is no longer needed"]
pub fn remap_address(
    map: &PhysicalMemoryMap,
    new_size: u64,
    flags: PageTableFlags,
) -> Result<PhysicalMemoryMap, MapError> {
    unmap_address(*map);
    map_address(map.phys_addr, new_size, flags)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MapError {
    #[error("Unable to allocate virtual memory")]
    UnableToAllocateVirtualMemory,
    #[error("Mapping error")]
    MappingError,
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
        let map = super::map_address(
            frame.start_address(),
            4096,
            x86_64::structures::paging::PageTableFlags::PRESENT
                | x86_64::structures::paging::PageTableFlags::WRITABLE,
        )
        .unwrap();
        //Check that the mapping is correct
        assert_eq!(map.phys_addr(), frame.start_address());
        //Unmap it
        super::unmap_address(map);
    }
}
