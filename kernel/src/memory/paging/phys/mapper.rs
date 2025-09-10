use alloc::vec::Vec;
use limine::{
    memory_map::{Entry, EntryType},
    response::MemoryMapResponse,
};
use log::{debug, error, info, trace};
use x86_64::{
    structures::paging::{
        mapper::{MapToError, UnmapError},
        page::PageRangeInclusive,
        FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, PhysFrame, Size4KiB,
    },
    PhysAddr,
};

use crate::memory::{
    self,
    paging::{KernelPage, KernelPhysFrame},
    req_data::MemoryMap,
};

pub(crate) struct PageFrameAllocator {
    map: &'static MemoryMap,
    off: usize,
    current: Entry,
    entry_offset: u64,
    unused: Vec<KernelPhysFrame>,
}

impl PageFrameAllocator {
    pub fn new(map: &'static MemoryMap) -> Self {
        // Find the first usable entry
        let (current, entry) = map
            .iter()
            .enumerate()
            .find(|e| e.1.entry_type == EntryType::USABLE)
            .expect("No usable memory regions found")
            .clone();

        debug!("Frames in memory map {{ ");
        for entry in map.iter() {
            debug!(
                "  [{:x}]({:x}): {}",
                entry.base,
                entry.length,
                Self::fmt_entry_type(entry)
            );
        }
        debug!("}}");

        Self {
            map,
            off: current + 1,
            current: *entry,
            entry_offset: 0,
            // This is okay before we have a heap because this will not allocate until anything is pushed to it
            unused: Vec::new(),
        }
    }

    fn next_usable(&mut self) -> Option<(usize, Entry)> {
        let mmr = self
            .map
            .iter()
            .enumerate()
            .skip(self.off)
            .find(|e| e.1.entry_type == EntryType::USABLE)
            .map(|(i, e)| (i, *e));
        if let Some((off, entry)) = mmr {
            self.off = off + 1;
            info!(
                "Found memory chunk {}[{:x}]({:x}): {}",
                off,
                entry.base,
                entry.length,
                Self::fmt_entry_type(&entry)
            );
        }
        mmr
    }

    pub unsafe fn map_range(
        &mut self,
        page_range: PageRangeInclusive<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
    ) -> Result<(), MapError> {
        let mut mapper = memory::paging::OFFSET_PAGE_TABLE.get();
        unsafe { self.map_range_pagetable(page_range, flags, &mut *mapper) }
    }

    pub unsafe fn map_range_pagetable(
        &mut self,
        page_range: PageRangeInclusive<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
        pagetable: &mut OffsetPageTable,
    ) -> Result<(), MapError> {
        for page in page_range {
            let frame = self.allocate_frame().ok_or(MapError::NoUsableMemory)?;
            unsafe {
                pagetable.map_to(page, frame, flags, &mut *self)?.flush();
            }
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

    fn fmt_entry_type(entry: &Entry) -> &'static str {
        match entry.entry_type {
            EntryType::USABLE => "USABLE",
            EntryType::RESERVED => "RESERVED",
            EntryType::ACPI_RECLAIMABLE => "ACPI_RECLAIMABLE",
            EntryType::ACPI_NVS => "ACPI_NVS",
            EntryType::BAD_MEMORY => "BAD_MEMORY",
            EntryType::BOOTLOADER_RECLAIMABLE => "BOOTLOADER_RECLAIMABLE",
            EntryType::EXECUTABLE_AND_MODULES => "EXECUTABLE_AND_MODULES",
            EntryType::FRAMEBUFFER => "FRAMEBUFFER",
            _ => "UNKNOWN",
        }
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
    fn allocate_frame(&mut self) -> Option<KernelPhysFrame> {
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
            let (_, entry) = self.next_usable()?;
            self.current = entry;
            self.entry_offset = 0;
            self.allocate_frame()
        }
    }
}

impl FrameDeallocator<Size4KiB> for PageFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: KernelPhysFrame) {
        if memory::is_initialized() {
            self.unused.push(frame);
        } else {
            error!("Attempted to deallocate frame before memory initialized");
        }
    }
}
