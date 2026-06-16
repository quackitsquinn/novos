use cake::log::{debug, info};

use crate::{
    MapFlags, MemError, VirtualMemoryRange,
    arch::{
        self,
        x86_64::{mapper::Mapper, set_mapper},
    },
    entry_walker::EntryWalker,
    paging::{
        AddressExt, Large, Medium, Page, PageTable, PageTableIndex, PhysAddr, PrimitiveSize, Small,
        VirtAddr, map_primitive,
    },
};

pub(crate) unsafe fn init_unchecked(
    root: &'static mut PageTable,
    offset: VirtAddr,
    mut walker: EntryWalker<'static>,
    scratch_range: VirtualMemoryRange,
) -> Result<(), MemError> {
    if scratch_range.size < arch::L1_PAGE_SIZE as usize * 16 {
        return Err(MemError::ScratchSpaceTooSmall {
            provided: scratch_range.size as u64,
            required: arch::L1_PAGE_SIZE as u64 * 16,
        });
    }

    // Map the first page of the scratch range to test that the mapper is working correctly.
    type Size = Large; // We can use a medium page here since the scratch range is guaranteed to be large enough to hold at least one medium page, and using a larger page size will allow us to test that the mapper can handle mapping larger pages correctly.
    let test_page =
        Page::<Size>::containing_address(scratch_range.base).ok_or(MemError::OutOfMemory)?;
    let test_frame = walker.next_frame::<Size>().ok_or(MemError::OutOfMemory)?;

    map_primitive(test_frame, test_page, MapFlags::WRITABLE, &mut walker)?;

    info!(
        "Successfully mapped first page of scratch range. Writing to it to test that the mapping is working correctly..."
    );
    // Write to the mapped page to test that the mapping is working correctly.
    unsafe {
        *test_page
            .start_address()
            .as_mut_ptr::<[u8; Size::SIZE as usize]>() = [0xAAu8; Size::SIZE as usize];
    }

    info!("Attempting to unmap the test page");
    unsafe {
        let (frame, flush) = arch::unmap_primitive(test_page)?;
        flush.flush();
        debug!("Successfully unmapped test page, got frame {:?}", frame);
    }

    // Initialize the mapper and set it as the active mapper for the system. T
    // his is necessary to perform any virtual memory operations, including mapping the scratch space.
    let mapper = unsafe { Mapper::new_offset(root, offset) };
    unsafe { set_mapper(mapper) };

    info!("Mapping first page of scratch range to test map_primitive...");

    // let mut bitmap = unsafe { Bitmap::init(u64_slice, scratch_pages as u64, scratch_range.base) };

    // Now that we have the scratch space mapped and the bitmap initialized, we can mark the pages we just mapped as allocated in the bitmap, since they are now in use by the memory manager.
    // unsafe { bitmap.set(BitPtr::new(0, 0), needed_pages as u64) }; // Mark the pages we just mapped as allocated in the bitmap.

    //let _ = register_global_bitmap(bitmap); // The user is allowed to register their own bitmap if they want.

    Ok(())
}

pub(crate) unsafe fn init_load_recursive(
    _root: &'static mut PageTable,
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
