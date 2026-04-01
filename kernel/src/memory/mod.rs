use core::{convert::Infallible, mem};

use cake::log::info;
use nmm::{
    VirtualMemoryRange,
    arch::{HIGHER_HALF_START, x86_64::VirtAddr},
};
use x86_64::{
    VirtAddr as XVirtAddr,
    registers::control::Cr3,
    structures::paging::{Page, PageTableFlags, page::PageRangeInclusive},
};

use crate::{
    declare_module,
    memory::paging::{KernelPageSize, map::map},
    requests::{KERNEL_ELF, MEMORY_MAP, PHYSICAL_MEMORY_OFFSET},
};

pub mod allocator;
pub mod elf_req_data;
pub mod paging;
pub mod req_data;

/// Enables or disables allocation debugging based on the ALLOC_DEBUG environment variable.
pub const ALLOC_DEBUG: bool = option_env!("ALLOC_DEBUG").is_some();

declare_module!("memory", init);

fn init() -> Result<(), Infallible> {
    let hhdm_offset = *PHYSICAL_MEMORY_OFFSET
        .get()
        .expect("Physical memory offset not provided by bootloader");
    let cr3 = Cr3::read();
    let pml4_vaddr = (hhdm_offset + cr3.0.start_address().as_u64()) as *mut ();
    let memory_map = MEMORY_MAP.lock_limine();
    let memory_map = memory_map.entries();
    info!(
        "Initializing nmm [hhdm_mapping: {:x}, pml4: {}, managed_range: {:?}]",
        hhdm_offset,
        pml4_vaddr as u64,
        map::nmm_managed_range::RANGE
    );
    unsafe {
        nmm::init(
            pml4_vaddr,
            VirtAddr::new_truncate(hhdm_offset),
            mem::transmute(memory_map),
            map::nmm_managed_range::RANGE,
        )
    }
    .expect("Failed to initialize memory manager");
    info!("Memory manager initialized");
    init_heap();
    Ok(())
}

fn init_heap() {}

/// Configure a heap allocator with the given name, allocator function, and heap size.
/// alloc_fn should be a function that takes two usize arguments: the start and end of the heap (in that order).
fn configure_heap_allocator(
    alloc_name: &str,
    alloc_fn: unsafe fn(*mut u8, *mut u8),
    heap_start: VirtAddr,
    heap_size: u64,
) {
    // let heap_end = heap_start + heap_size;
    // let heap_start_page = Page::containing_address(heap_start);
    // let heap_end_page = Page::containing_address(heap_end);
    // let heap_range: PageRangeInclusive<KernelPageSize> =
    //     Page::range_inclusive(heap_start_page, heap_end_page - 1);

    // info!(
    //     "{} Heap range: 0x{:x} - 0x{:x} ({:?} - {:?}: {} pages)",
    //     alloc_name,
    //     heap_start,
    //     heap_end,
    //     heap_start_page,
    //     heap_end_page,
    //     heap_range.len()
    // );

    // unsafe {
    //     paging::phys::FRAME_ALLOCATOR.get().map_range(
    //         heap_range,
    //         PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    //     )
    // }
    // .expect("Unable to map heap");

    // info!(
    //     "{} Heap initialized at 0x{:x} - 0x{:x}",
    //     alloc_name, heap_start, heap_end
    // );
    // info!("Initializing {} allocator", alloc_name);
    // unsafe { alloc_fn(heap_start.as_mut_ptr(), heap_end.as_mut_ptr()) };
    // info!("{} allocator initialized", alloc_name);
}
