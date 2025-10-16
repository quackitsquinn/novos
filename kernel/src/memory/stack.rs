//! Stack management and abstractions.
use cake::log::info;
use x86_64::{
    VirtAddr,
    structures::paging::{Mapper, Page, PageTableFlags, Size4KiB, mapper::MapToError},
};

use crate::memory::paging::KernelPage;

use super::paging::{
    phys::{FRAME_ALLOCATOR, mapper::MapError},
    vaddr_mapper::VIRT_MAPPER,
};

/// Represents a stack in the system.
#[derive(Debug, Clone, Copy)]
pub struct Stack {
    start_page: Page,
    end_page: Page,
}

impl Stack {
    /// Creates a new stack with the given base address and size.
    ///
    /// # Safety
    ///
    /// The caller must ensure the page range is valid and not overlapping with other memory regions.
    pub unsafe fn new(start: &Page, end: &Page) -> Self {
        let start_page = *start;
        let end_page = *end;

        Self {
            start_page,
            end_page,
        }
    }

    /// Allocates a new stack with the given size and base page.
    ///
    /// # Arguments
    ///
    /// * `offset_table` - The page table mapper to use for mapping pages.
    /// * `size` - The size of the stack in bytes.
    /// * `base` - The base page of the stack. This is the lowest address of the stack.
    /// * `stack_flags` - The flags to use for the stack pages.
    ///
    /// # Returns
    ///
    /// A Result containing the newly allocated Stack or a MapError if the allocation failed.
    pub fn allocate_stack<T: Mapper<Size4KiB>>(
        mapper: &mut T,
        size: u64,
        base: Page,
        stack_flags: StackFlags,
    ) -> Result<Self, MapError> {
        let range = Page::range_inclusive(base, base + size);
        info!("Allocating stack: {:#x?} - {:#x?}", base, base + size);
        unsafe {
            FRAME_ALLOCATOR
                .get()
                .map_range_pagetable(range, stack_flags.flags(), mapper)?;
        }

        let stack = unsafe { Self::new(&base, &(base + size)) };
        Ok(stack)
    }

    /// Creates a new kernel stack with the given size and start page.
    ///
    /// # Arguments
    ///
    /// * `size` - The size of the stack in bytes.
    /// * `start_page` - The start page of the stack.
    /// * `stack_flags` - The flags to use for the stack pages.
    ///
    /// # Returns
    ///
    /// A Result containing the newly created Stack or a MapError if the creation failed.
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

    /// Allocates a new kernel stack with the given size and stack flags.
    pub fn allocate_kernel_stack(size: u64, stack_flags: StackFlags) -> Result<Self, MapError> {
        assert!(size % 4096 == 0, "Stack size must be a multiple of 4096");
        // Map enough pages for the stack
        let range = VIRT_MAPPER
            .get()
            .allocate(size / 4096)
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

    /// Deallocates the stack using the given page table mapper.
    /// # Safety
    /// The caller must ensure that the page table mapper is the same one that was used to allocate the stack.
    pub unsafe fn deallocate_stack(
        self,
        mut mapper: impl Mapper<Size4KiB>,
    ) -> Result<(), MapError> {
        let range = Page::range_inclusive(self.start_page, self.end_page);
        unsafe {
            FRAME_ALLOCATOR
                .get()
                .unmap_range_pagetable(range, &mut mapper)
        }
    }

    /// Returns the stack base address.
    /// This is not the base address of the stack, but the address that would be used as the base address for the stack pointer.
    pub fn get_stack_base(&self) -> VirtAddr {
        self.end_page.start_address() + 0x1000
    }
}

/// Flags for configuring the properties of a stack.
#[derive(Debug, Clone, Copy)]
pub enum StackFlags {
    /// Read-write stack accessible only in kernel space.
    KernelMode,
    /// Read-write stack accessible from both userspace and kernel space.
    UserMode,
    /// Custom flags for the stack pages.
    Custom(PageTableFlags),
}

impl StackFlags {
    /// Returns the page table flags for the stack.s
    pub fn flags(&self) -> PageTableFlags {
        match self {
            StackFlags::KernelMode => PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            StackFlags::UserMode => {
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
            }
            StackFlags::Custom(flags) => *flags,
        }
    }
}

impl Default for StackFlags {
    fn default() -> Self {
        StackFlags::KernelMode
    }
}
