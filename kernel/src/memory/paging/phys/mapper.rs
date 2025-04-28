use alloc::vec::Vec;
use limine::{
    memory_map::{Entry, EntryType},
    response::MemoryMapResponse,
};
use log::error;
use x86_64::{
    structures::paging::{
        mapper::{MapToError, UnmapError},
        page::PageRangeInclusive,
        FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame,
        Size4KiB,
    },
    PhysAddr,
};

use crate::memory;

pub struct PageFrameAllocator {
    map: &'static MemoryMapResponse,
    off: usize,
    current: Entry,
    entry_offset: u64,
    unused: Vec<PhysFrame<Size4KiB>>,
}

impl PageFrameAllocator {
    pub fn new(map: &'static MemoryMapResponse) -> Self {
        // Find the first usable entry
        let (current, entry) = map
            .entries()
            .iter()
            .enumerate()
            .find(|e| e.1.entry_type == EntryType::USABLE)
            .expect("No usable memory regions found")
            .clone();

        Self {
            map,
            off: current,
            current: **entry,
            entry_offset: 0,
            // This is okay before we have a heap because this will not allocate until anything is pushed to it
            unused: Vec::new(),
        }
    }

    fn next_usable(&mut self) -> Option<(usize, Entry)> {
        self.map
            .entries()
            .iter()
            .skip(self.off)
            .enumerate()
            .find(|e| e.1.entry_type == EntryType::USABLE)
            .map(|(i, e)| (i, **e))
    }

    ///  Maps a page to the kernel's page table
    pub unsafe fn map_page(
        &mut self,
        page: Page<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
    ) -> Result<(), MapError> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        unsafe { self.map_page_pagetable(page, flags, &mut *mapper) }
    }

    /// Maps a page to the kernel's page table using the provided page table
    pub unsafe fn map_page_pagetable(
        &mut self,
        page: Page<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
        pagetable: &mut OffsetPageTable,
    ) -> Result<(), MapError> {
        unsafe {
            pagetable
                .map_to(
                    page,
                    self.allocate_frame().expect("Unable to map frame"),
                    flags,
                    &mut *self,
                )?
                .flush();
        }
        Ok(())
    }

    /// Maps a page to the kernel's page table using the provided frame
    pub unsafe fn map_to(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
    ) -> Result<(), MapError> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        unsafe { self.map_to_pagetable(page, frame, flags, &mut *mapper) }
    }

    /// Maps a page to the given page table using the provided frame
    pub unsafe fn map_to_pagetable(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
        pagetable: &mut OffsetPageTable,
    ) -> Result<(), MapError> {
        unsafe {
            pagetable.map_to(page, frame, flags, &mut *self)?.flush();
        }
        Ok(())
    }

    pub unsafe fn map_range(
        &mut self,
        page_range: PageRangeInclusive<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
    ) -> Result<(), MapError> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        unsafe { self.map_range_pagetable(page_range, flags, &mut *mapper)? }
        Ok(())
    }

    pub unsafe fn map_range_pagetable(
        &mut self,
        page_range: PageRangeInclusive<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
        pagetable: &mut OffsetPageTable,
    ) -> Result<(), MapError> {
        for page in page_range {
            unsafe {
                pagetable
                    .map_to(
                        page,
                        self.allocate_frame().ok_or(MapError::NoUsableMemory)?,
                        flags,
                        &mut *self,
                    )
                    .map(|flush| flush.flush())?;
            }
        }
        Ok(())
    }

    pub unsafe fn unmap_page(&mut self, page: Page<Size4KiB>) -> Result<(), MapError> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        unsafe { self.unmap_page_pagetable(page, &mut *mapper) }
    }

    pub unsafe fn unmap_page_pagetable(
        &mut self,
        page: Page<Size4KiB>,
        pagetable: &mut OffsetPageTable,
    ) -> Result<(), MapError> {
        unsafe {
            let (frame, flush) = pagetable.unmap(page)?;
            flush.flush();
            self.deallocate_frame(frame);
        }
        Ok(())
    }

    pub unsafe fn unmap_range(
        &mut self,
        page_range: PageRangeInclusive<Size4KiB>,
    ) -> Result<(), MapError> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        unsafe { self.unmap_range_pagetable(page_range, &mut *mapper) }
    }

    pub unsafe fn unmap_range_pagetable(
        &mut self,
        page_range: PageRangeInclusive<Size4KiB>,
        pagetable: &mut OffsetPageTable,
    ) -> Result<(), MapError> {
        for page in page_range {
            unsafe {
                let (frame, flush) = pagetable.unmap(page)?;
                flush.flush();
                self.deallocate_frame(frame);
            }
        }
        Ok(())
    }

    pub fn is_page_mapped(&mut self, page: Page<Size4KiB>) -> Option<PhysFrame<Size4KiB>> {
        let mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        mapper.translate_page(page).ok()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MapError {
    #[error("No usable memory regions found")]
    NoUsableMemory,
    #[error("Unable to map frame: {:?}", .0)]
    MapError(MapToError<Size4KiB>),
    #[error("Unable to unmap frame: {:?}", .0)]
    UnmapError(UnmapError),
}

// This is a workaround for the fact that for some reason MapToError does not implement Error
impl From<MapToError<Size4KiB>> for MapError {
    fn from(err: MapToError<Size4KiB>) -> Self {
        MapError::MapError(err)
    }
}

impl From<UnmapError> for MapError {
    fn from(err: UnmapError) -> Self {
        MapError::UnmapError(err)
    }
}

unsafe impl FrameAllocator<Size4KiB> for PageFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if self.unused.len() > 0 {
            return self.unused.pop();
        }
        let current_end = self.current.base + self.current.length;
        if current_end >= self.current.base + self.entry_offset + 4096 {
            let frame =
                PhysFrame::containing_address(PhysAddr::new(self.current.base + self.entry_offset));
            self.entry_offset += 4096;
            Some(frame)
        } else {
            let (off, entry) = self.next_usable()?;
            self.off = off;
            self.current = entry;
            self.entry_offset = 0;
            self.allocate_frame()
        }
    }
}

impl FrameDeallocator<Size4KiB> for PageFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        if memory::is_initialized() {
            self.unused.push(frame);
        } else {
            error!("Attempted to deallocate frame before memory initialized");
        }
    }
}
