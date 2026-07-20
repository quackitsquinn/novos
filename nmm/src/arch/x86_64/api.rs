use cake::log::{debug, info};
use x86_64::registers::control::Cr3;

use crate::{
    MapFlags, MemError, align,
    arch::{self, L1_PAGE_SIZE, pml4_phys, x86_64::mapper::Mapper},
    bitmap::{BitPtr, Bitmap, PhysicalMemoryManager, VirtualMemoryManager},
    entry_walker::EntryWalker,
    paging::{
        Address, AddressExt, EntryMappingFlags, FragmentSize, Frame, Medium, MemoryFragment, Page,
        PageTable, PageTableIndex, PhysAddr, Small, VirtAddr,
        asm::{self, AddressSpace},
        map_from, map_primitive,
        primitives::MemoryRange,
    },
};

pub(crate) unsafe fn init_unchecked(
    offset: VirtAddr,
    mut walker: EntryWalker<'static>,
    scratch_range: MemoryRange<VirtAddr>,
) -> Result<(), MemError> {
    if scratch_range.size() < arch::L1_PAGE_SIZE * 16 {
        return Err(MemError::ScratchSpaceTooSmall {
            provided: scratch_range.size() as u64,
            required: arch::L1_PAGE_SIZE as u64 * 16,
        });
    }

    let cr3: Frame<Small> = pml4_phys();
    let root: &'static mut PageTable = unsafe {
        &mut *(cr3
            .translate_offset(offset)
            .unwrap()
            .as_mut_ptr::<PageTable>())
    };

    for (i, entry) in root.entries().chunks_exact(4).enumerate() {
        debug!(
            "pml4[{}..{}]: {} {} {} {}",
            i * 4,
            (i * 4) + 4,
            entry[0],
            entry[1],
            entry[2],
            entry[3]
        );
    }

    // Initialize the mapper and set it as the active mapper for the system.
    // This is necessary to perform any virtual memory operations, including mapping the scratch space.
    let mapper = unsafe { Mapper::new_offset(root, offset) };
    unsafe {
        asm::set_active(AddressSpace::without_vmm(mapper, cr3));
    };

    info!("Found {} bytes of usable memory", walker.usable_memory());

    // Always make sure the number of bytes will be aligned to u64
    let n_pages = (scratch_range.size() as u64).div_ceil(L1_PAGE_SIZE);
    let n_bytes = align!(up, n_pages / 8, core::mem::size_of::<u64>() as u64);
    let n_entries = n_bytes / core::mem::size_of::<u64>() as u64;

    info!(
        "Mapping scratch space: base={:#x}, size={} bytes, pages={}, entries={}",
        scratch_range.start().as_u64(),
        n_bytes,
        n_pages,
        n_entries
    );
    unsafe {
        map_from(
            scratch_range.start(),
            n_bytes,
            MapFlags::WRITABLE,
            EntryMappingFlags::empty(),
            &mut walker,
        )?;
    }

    let entries = unsafe {
        core::slice::from_raw_parts_mut(
            scratch_range.start().as_mut_ptr::<u64>(),
            n_entries as usize,
        )
    };

    info!("Initializing virtual memory manager with scratch space");
    let mut vmm = unsafe { VirtualMemoryManager::init(entries, scratch_range) };
    unsafe { vmm.mark_allocated(scratch_range.start(), n_bytes) }

    info!("Initializing physical memory manager with scratch space");
    let pmm = unsafe { PhysicalMemoryManager::init(walker, &mut vmm)? };
    info!("Physical memory manager initialized successfully");

    {
        let ads = asm::active();
        ads.set_vmm(vmm);
    }
    asm::set_physical_memory_manager(pmm);

    info!("Memory manager initialized successfully");
    //Err(MemError::Uninit("todo"))
    Ok(())
}

pub(crate) unsafe fn init_load_recursive(
    _root: &'static mut PageTable,
    _index: PageTableIndex,
    _phys_addr: PhysAddr,
) -> Result<(), MemError> {
    todo!("todo")
}
