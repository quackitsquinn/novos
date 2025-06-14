use log::info;
use x86_64::{
    structures::paging::{mapper::MapToError, OffsetPageTable, Page, PageTableFlags},
    VirtAddr,
};

use crate::memory::paging::KernelPage;

use super::paging::{
    phys::{mapper::MapError, FRAME_ALLOCATOR},
    virt::VIRT_MAPPER,
};

/// Represents a stack in the system.
#[derive(Debug, Clone, Copy)]
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
        stack_flags: StackFlags,
    ) -> Result<Self, MapError> {
        let range = Page::range_inclusive(base, base + size);
        info!("Allocating stack: {:#x?} - {:#x?}", base, base + size);
        unsafe {
            FRAME_ALLOCATOR
                .get()
                .map_range_pagetable(range, stack_flags.flags(), offset_table)?;
        }

        let stack = unsafe { Self::new(&base, &(base + size)) };
        Ok(stack)
    }

    pub fn create_kernel_stack(
        size: u64,
        start_page: Page,
        stack_flags: StackFlags,
    ) -> Result<Self, MapError> {
        let end_page = Page::containing_address(start_page.start_address() + size);
        let range = Page::range_inclusive(start_page, end_page);
        info!(
            "Allocating kernel stack: {:#x?} - {:#x?}",
            start_page, end_page
        );
        unsafe {
            FRAME_ALLOCATOR
                .get()
                .map_range(range, stack_flags.flags())?;
        }

        let stack = unsafe { Self::new(&start_page, &end_page) };
        Ok(stack)
    }

    pub fn allocate_kernel_stack(size: u64, stack_flags: StackFlags) -> Result<Self, MapError> {
        // Map enough pages for the stack
        let range = VIRT_MAPPER
            .get()
            .allocate(size)
            .ok_or(MapError::NoUsableMemory)?;
        let start_page = Page::containing_address(range.start);
        let end_page: KernelPage = Page::containing_address(range.end());
        info!(
            "Allocating kernel stack: {:#x?} - {:#x?}",
            start_page, end_page
        );
        let res = Self::create_kernel_stack(size, start_page, stack_flags);
        match res {
            Err(MapError::MapError(MapToError::PageAlreadyMapped(_))) => {
                Self::allocate_kernel_stack(size, stack_flags)
            }
            _ => res,
        }
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

#[derive(Debug, Clone, Copy)]
pub enum StackFlags {
    RWKernel,
    RWUser,
    Custom(PageTableFlags),
}

impl StackFlags {
    pub fn flags(&self) -> PageTableFlags {
        match self {
            StackFlags::RWKernel => PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            StackFlags::RWUser => {
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
            }
            StackFlags::Custom(flags) => *flags,
        }
    }
}

impl Default for StackFlags {
    fn default() -> Self {
        StackFlags::RWKernel
    }
}
