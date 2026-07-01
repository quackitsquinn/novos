use cake::log::{debug, info};

use crate::{
    MapFlags, MemError, VirtualMemoryRange, align,
    arch::{
        self, L1_PAGE_SIZE,
        x86_64::{mapper::Mapper, set_mapper},
    },
    bitmap::{BitPtr, Bitmap},
    entry_walker::EntryWalker,
    paging::{
        Address, AddressExt, FragmentSize, Medium, MemoryFragment, Page, PageTable, PageTableIndex,
        PhysAddr, VirtAddr, map_from, map_primitive,
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

    // Initialize the mapper and set it as the active mapper for the system.
    // This is necessary to perform any virtual memory operations, including mapping the scratch space.
    let mapper = unsafe { Mapper::new_offset(root, offset) };
    unsafe { set_mapper(mapper) };

    // Always make sure the number of bytes will be aligned to u64
    let n_pages = (scratch_range.size as u64).div_ceil(L1_PAGE_SIZE);
    let n_bytes = align!(up, n_pages / 8, core::mem::size_of::<u64>() as u64);
    let n_entries = n_bytes / core::mem::size_of::<u64>() as u64;

    info!(
        "Mapping scratch space: base={:#x}, size={} bytes, pages={}, entries={}",
        scratch_range.base.as_u64(),
        n_bytes,
        n_pages,
        n_entries
    );
    unsafe {
        map_from(scratch_range.base, n_bytes, MapFlags::WRITABLE, &mut walker)?;
    }

    let entries = unsafe {
        core::slice::from_raw_parts_mut(scratch_range.base.as_mut_ptr::<u64>(), n_entries as usize)
    };
    let mut bitmap = Bitmap::init(entries, n_pages, scratch_range.base.as_u64());

    // Now that we have the scratch space mapped and the bitmap initialized, we can mark the pages we just mapped as allocated in the bitmap,
    // since they are now in use by the memory manager.
    bitmap.set(BitPtr::new(0, 0), n_pages as u64); // Mark the pages we just mapped as allocated in the bitmap.

    panic!("woah!");
    Ok(())
}

pub(crate) unsafe fn init_load_recursive(
    _root: &'static mut PageTable,
    _index: PageTableIndex,
    _phys_addr: PhysAddr,
) -> Result<(), MemError> {
    todo!("todo")
}
