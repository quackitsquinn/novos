use x86_64::{
    structures::paging::{page::PageRangeInclusive, OffsetPageTable, Page, PageTableFlags},
    VirtAddr,
};

use super::paging::phys::{
    mapper::{MapError, PageFrameAllocator},
    FRAME_ALLOCATOR,
};

pub struct Stack {
    pub stack_base: VirtAddr,
    pub start_page: Page,
    pub end_page: Page,
}

impl Stack {
    /// Creates a new stack with the given base address and size.
    ///
    /// # Safety
    ///
    /// The caller must ensure the page range is valid and not overlapping with other memory regions.
    pub unsafe fn new(start: &Page, end: &Page) -> Self {
        let stack_base = start.start_address();
        let start_page = *start;
        let end_page = *end;

        Self {
            stack_base,
            start_page,
            end_page,
        }
    }

    pub fn allocate_stack(
        offset_table: &mut OffsetPageTable,
        size: u64,
        base: Page,
    ) -> Result<Self, MapError> {
        let range = Page::range_inclusive(base, base + size);
        unsafe {
            FRAME_ALLOCATOR.get().map_range_pagetable(
                range,
                PageTableFlags::PRESENT,
                offset_table,
            )?;
        }

        let stack = unsafe { Self::new(&base, &(base + size)) };
        Ok(stack)
    }

    pub fn allocate_kernel_stack(size: u64, base: Page) -> Result<Self, MapError> {
        let range = Page::range_inclusive(base, base + size);
        unsafe {
            FRAME_ALLOCATOR
                .get()
                .map_range(range, PageTableFlags::PRESENT)?;
        }

        let stack = unsafe { Self::new(&base, &(base + size)) };
        Ok(stack)
    }

    pub fn deallocate_stack(self, offset_table: &mut OffsetPageTable) -> Result<(), MapError> {
        let range = Page::range_inclusive(self.start_page, self.end_page);
        unsafe {
            FRAME_ALLOCATOR
                .get()
                .unmap_range_pagetable(range, offset_table)
        }
    }

    pub fn deallocate_kernel_stack(self) -> Result<(), MapError> {
        let range = Page::range_inclusive(self.start_page, self.end_page);
        unsafe { FRAME_ALLOCATOR.get().unmap_range(range) }
    }

    /// Returns the stack base address.
    /// This is not the base address of the stack, but the address that would be used as the base address for the stack pointer.
    pub fn get_stack_base(&self) -> VirtAddr {
        self.end_page.start_address() + 0x1000
    }
}
