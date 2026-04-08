use core::slice;

use ::x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, Size4KiB,
    mapper::MapToError,
};
use cake::{limine::memory_map, log::debug};
use cfg_if::cfg_if;

use crate::{
    MapFlags, MemError, VirtualMemoryRange,
    arch::{
        PhysAddr, VirtAddr,
        x86_64::{self, ArchError},
    },
    bitmap::{BitPtr, Bitmap, register_global_bitmap},
    entry_walker::EntryWalker,
    paging::PageTableIndex,
};

pub(crate) type Offset<'a> = OffsetPageTable<'a>;

pub(crate) unsafe fn init_unchecked(
    root: *mut (),
    offset: VirtAddr,
    mut ranges: EntryWalker<'static>,
    scratch_range: VirtualMemoryRange,
) -> Result<(), MemError> {
    if scratch_range.size < super::TABLE_SIZE * 16 {
        return Err(MemError::ScratchSpaceTooSmall {
            provided: scratch_range.size as u64,
            required: super::TABLE_SIZE as u64 * 16,
        });
    }

    // First, we need to bootstrap the virtual memory system by allocating however many pages we need to manage the scratch range.
    let scratch_pages = scratch_range.size / super::TABLE_SIZE; // We can safely do this, since it's up to the caller to make sure the size is page-aligned.
    let needed_pages = scratch_pages.div_ceil(Bitmap::MEMORY_PER_PAGE as usize);
    let n_entries = (scratch_pages as usize).div_ceil(Bitmap::PAGES_PER_ENTRY as usize);
    let pml4 = unsafe { &mut *(root as *mut PageTable) };
    let mut offset_table = unsafe { Offset::new(pml4, *offset) };
    let slice_base: *mut u64 = scratch_range.base.as_mut_ptr();
    let mut next_page = *scratch_range.base;

    debug!(
        "NMM: mapping {} pages for scratch space [base: {:x}, size: {:x}]",
        needed_pages, next_page, scratch_range.size
    );
    for _ in 0..needed_pages {
        let frame = ranges.allocate_frame().ok_or(MemError::OutOfMemory)?;
        unsafe {
            let flush = offset_table.map_to(
                Page::<Size4KiB>::from_start_address(next_page).unwrap(),
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                &mut ranges,
            )?;

            cfg_if::cfg_if! {
                if #[cfg(target_arch = "x86_64")] {
                    flush.flush();
                } else {
                    flush.ignore();
                }
            };
        };

        next_page += super::TABLE_SIZE as u64;
    }

    let u64_slice = unsafe { slice::from_raw_parts_mut(slice_base, n_entries) };

    let mut bitmap = unsafe { Bitmap::init(u64_slice, scratch_pages as u64, scratch_range.base) };

    // Now that we have the scratch space mapped and the bitmap initialized, we can mark the pages we just mapped as allocated in the bitmap, since they are now in use by the memory manager.
    unsafe { bitmap.set(BitPtr::new(0, 0), needed_pages as u64) }; // Mark the pages we just mapped as allocated in the bitmap.

    let _ = register_global_bitmap(bitmap); // The user is allowed to register their own bitmap if they want.

    Ok(())
}

pub(crate) unsafe fn init_load_recursive(
    _root: *mut (),
    _index: PageTableIndex,
    _phys_addr: PhysAddr,
) -> Result<(), MemError> {
    todo!("todo")
}

pub(crate) unsafe fn map_unchecked(
    _virt_base: VirtAddr,
    _phys_base: PhysAddr,
    _byte_size: usize,
    _flags: MapFlags,
) -> Result<(), MemError> {
    todo!()
}

pub(crate) unsafe fn unmap_unchecked(
    _virt_base: VirtAddr,
    _byte_size: usize,
) -> Result<(), MemError> {
    todo!()
}

/// Maps `byte_size` bytes of memory, returning the base virtual address of the mapped region. The physical memory for this mapping is allocated by the memory manager, and the mapping is created with the specified flags.
#[must_use = "The returned virtual address must be freed with `unmap` when it is no longer needed to avoid memory leaks and ensure proper resource management."]
pub(crate) unsafe fn alloc_paged(
    _byte_size: usize,
    _flags: MapFlags,
) -> Result<VirtAddr, MemError> {
    todo!()
}
