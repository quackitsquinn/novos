use x86_64::{
    structures::paging::{mapper::MapToError, Page, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

use super::FRAME_ALLOCATOR;

/// A physical memory mapping. The mapping is valid for `size` bytes starting at `phys`.
pub struct PhysicalMap {
    /// The physical address of the start of the mapping.
    phys: PhysAddr,
    /// The size of the mapping. In bytes. The returned mapping must be >= this size.
    size: usize,
    /// The flags to use for the mapping.
    flags: PageTableFlags,
}

impl PhysicalMap {
    // Create a new physical map with the given physical address and size.
    pub unsafe fn new(phys: PhysAddr, size: usize, flags: PageTableFlags) -> Self {
        Self { phys, size, flags }
    }

    /// Map the physical memory.
    /// The physical memory is mapped to the given virtual address returned in the lock.
    pub unsafe fn map(self) -> Result<PhysicalMapLock, PhysMapError> {
        // TODO: Should we take ownership of the physical map? I think it's a good idea, because attempting to map the same physical memory twice would be a bug. (And not possible? I think? )
        let mut mapper = FRAME_ALLOCATOR.get();
        // First, we need to find how many pages we need to map. It will 90% just be one page, but we need to be sure.
        // If size is > 4096 we know we need to map at least 2 pages.
        let pages = if self.size % 4096 == 0 {
            self.size / 4096
        } else {
            self.size / 4096 + 1
        };

        let mut err = None;

        // Now attempt to identity map the physical memory.
        (0..pages)
            .map(|i| {
                let frame = PhysFrame::containing_address(PhysAddr::new(
                    self.phys.as_u64() + i as u64 * 4096,
                ));
                unsafe { mapper.identity_map(frame, self.flags) }
            })
            .map(|res| match res {
                Ok(flush) => Ok(flush),
                Err(e) => match e {
                    MapToError::PageAlreadyMapped(_) => Err(PhysMapError::AlreadyMapped),
                    _ => Err(PhysMapError::CouldNotMap),
                },
            })
            .for_each(|res| match res {
                Ok(flush) => flush.flush(),
                Err(e) => err = Some(e),
            });

        if err.is_some() {
            return Err(err.unwrap());
        }
        let phys = self.phys;
        Ok(unsafe { PhysicalMapLock::new(self, VirtAddr::new(phys.as_u64()), pages) })
    }
}

/// A RAII lock for a physical map. The physical map is unmapped when the lock is dropped.
pub struct PhysicalMapLock {
    map: PhysicalMap,
    virt: VirtAddr,
    page_count: usize,
}

impl PhysicalMapLock {
    /// Create a new physical map lock for the given physical map. The physical map is mapped to the given virtual address.
    unsafe fn new(map: PhysicalMap, virt: VirtAddr, page_count: usize) -> Self {
        Self {
            map,
            virt,
            page_count,
        }
    }

    /// Get the virtual address of the start of the mapping.
    pub fn virt(&self) -> VirtAddr {
        self.virt
    }

    pub fn contains(&self, addr: PhysAddr) -> bool {
        let start = self.map.phys - (self.map.phys.as_u64() % 4096);
        let end = start + self.page_count as u64 * 4096;
        addr >= start && addr < end
    }
}

impl Drop for PhysicalMapLock {
    fn drop(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PhysMapError {
    /// The physical memory is already mapped.
    #[error("The physical memory is already mapped")]
    AlreadyMapped,
    /// The physical memory could not be mapped.
    #[error("The physical memory could not be mapped")]
    CouldNotMap,
}
