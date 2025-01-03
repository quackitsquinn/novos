use alloc::vec::Vec;
use log::{error, info};

use x86_64::{
    structures::paging::{mapper::MapToError, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::util::OnceMutex;

use super::{FRAME_ALLOCATOR, OFFSET_PAGE_TABLE};

/// A physical memory mapping. The mapping is valid for `size` bytes starting at `phys`.
#[derive(Debug, Clone, Copy)]
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
    pub unsafe fn map(self) -> Result<PhysicalMapResult, PhysMapError> {
        // TODO: Should we take ownership of the physical map? I think it's a good idea, because attempting to map the same physical memory twice would be a bug. (And not possible? I think? )
        let mut mapper = OFFSET_PAGE_TABLE.get();
        let mut frame_allocator = FRAME_ALLOCATOR.get();

        if PHYSICAL_MAPS
            .get()
            .iter()
            .any(|(map, _)| map.phys == self.phys)
        {
            return Err(PhysMapError::AlreadyMapped);
        }
        // First, we need to find how many pages we need to map. It will 90% just be one page, but we need to be sure.
        // If size is > 4096 we know we need to map at least 2 pages.
        let pages = if self.size % 4096 == 0 {
            self.size / 4096
        } else {
            self.size / 4096 + 1
        };

        let page_offset = self.phys.as_u64() % 4096;

        let mut err = None;
        let mut virt = None;

        // Now attempt to identity map the physical memory.
        (0..pages)
            .map(|i| {
                let frame = PhysFrame::containing_address(PhysAddr::new(
                    self.phys.as_u64() + i as u64 * 4096,
                ));
                let page = next_page();
                if i == 0 {
                    virt = Some(page.start_address());
                }
                unsafe { mapper.map_to(page, frame, self.flags, &mut *frame_allocator) }
            })
            .map(|res| match res {
                Ok(flush) => Ok(flush),
                Err(e) => match e {
                    MapToError::PageAlreadyMapped(_) => Err(PhysMapError::AlreadyMapped),
                    _ => {
                        error!("Could not map physical memory: {:?}", e);
                        Err(PhysMapError::CouldNotMap)
                    }
                },
            })
            .for_each(|res| match res {
                Ok(flush) => flush.flush(),
                Err(e) => err = Some(e),
            });

        Ok(unsafe { PhysicalMapResult::new(self, virt.expect("no map") + page_offset, pages) })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PhysicalMapResult {
    map: PhysicalMap,
    virt: VirtAddr,
    page_count: usize,
}

impl PhysicalMapResult {
    /// Create a new physical map lock for the given physical map. The physical map is mapped to the given virtual address.
    unsafe fn new(map: PhysicalMap, virt: VirtAddr, page_count: usize) -> Self {
        let n = Self {
            map,
            virt,
            page_count,
        };
        PHYSICAL_MAPS
            .get()
            .push((map, Page::containing_address(virt)));
        n
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PhysMapError {
    /// The physical memory is already mapped.
    #[error("The physical memory is already mapped")]
    AlreadyMapped,
    /// The physical memory could not be mapped.
    #[error("The physical memory could not be mapped")]
    CouldNotMap,
}

static PHYSICAL_MAPS: OnceMutex<Vec<(PhysicalMap, Page<Size4KiB>)>> = OnceMutex::new();
static NEXT_PAGE: OnceMutex<Page<Size4KiB>> = OnceMutex::new();

pub(crate) fn init() {
    info!("Initializing physical memory mapping");
    // It is important that physical_maps is initialized after the heap, so ininting it here is the best option.
    PHYSICAL_MAPS.init(Vec::new());
    NEXT_PAGE.init(Page::containing_address(VirtAddr::new(
        super::super::MISC_MEM_OFFSET,
    )));
}

fn next_page() -> Page<Size4KiB> {
    let mut next_page = NEXT_PAGE.get();
    let page = *next_page;
    *next_page += 1;
    page
}
