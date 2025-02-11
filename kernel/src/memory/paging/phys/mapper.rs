use alloc::vec::Vec;
use limine::{
    memory_map::{Entry, EntryType},
    response::MemoryMapResponse,
};
use log::error;
use x86_64::{
    structures::paging::{
        mapper::MapToError, page::PageRangeInclusive, FrameAllocator, FrameDeallocator, Mapper,
        Page, PhysFrame, Size4KiB,
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

    pub fn map_page(
        &mut self,
        page: Page<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
    ) -> Result<(), MapToError<Size4KiB>> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        unsafe {
            mapper
                .map_to(
                    page,
                    self.allocate_frame().expect("Unable to map frame"),
                    flags,
                    &mut *self,
                )
                .map(|flush| flush.flush())
        }
    }

    pub fn map_to(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
    ) -> Result<(), MapToError<Size4KiB>> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut *self)
                .map(|flush| flush.flush())
        }
    }

    pub fn map_range(
        &mut self,
        page_range: PageRangeInclusive<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
    ) -> Result<(), MapToError<Size4KiB>> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        for page in page_range {
            unsafe {
                mapper
                    .map_to(
                        page,
                        self.allocate_frame().expect("Unable to map frame"),
                        flags,
                        &mut *self,
                    )
                    .map(|flush| flush.flush())?;
            }
        }
        Ok(())
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
