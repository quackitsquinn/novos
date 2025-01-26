use core::convert::Infallible;

use log::{info, trace};
use x86_64::{
    structures::paging::{page::PageRangeInclusive, Page, PageTableFlags, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::declare_module;

pub mod allocator;
pub mod paging;

// Evaluates to 0x4156_4F4E_0000
pub const HEAP_MEM_OFFSET: VirtAddr = VirtAddr::new((u32::from_ne_bytes(*b"NOVA") as u64) << 16);
pub const HEAP_SIZE: u64 = 1024 * 512; // 512 KiB

pub const TEST_HEAP_MEM_OFFSET: VirtAddr = VirtAddr::new(HEAP_MEM_OFFSET.as_u64() + HEAP_SIZE);
pub const TEST_HEAP_SIZE: u64 = HEAP_SIZE; // 512 KiB

declare_module!("memory", init);

fn init() -> Result<(), Infallible> {
    paging::MODULE.init();
    init_heap();
    paging::virt::MODULE.init();
    Ok(())
}

fn init_heap() {
    configure_heap_allocator("Kernel", allocator::init, HEAP_MEM_OFFSET, HEAP_SIZE);
    configure_heap_allocator(
        "Kernel Test",
        allocator::init_test,
        // Align the test heap to a 4 KiB boundary
        TEST_HEAP_MEM_OFFSET.align_up(4096u64),
        TEST_HEAP_SIZE,
    );
}

/// Configure a heap allocator with the given name, allocator function, and heap size.
/// alloc_fn should be a function that takes two usize arguments: the start and end of the heap (in that order).
fn configure_heap_allocator(
    alloc_name: &str,
    alloc_fn: unsafe fn(usize, usize),
    heap_start: VirtAddr,
    heap_size: u64,
) {
    let heap_end = heap_start + heap_size - 1u64;
    let heap_start_page = Page::containing_address(heap_start);
    let heap_end_page = Page::containing_address(heap_end);
    let heap_range: PageRangeInclusive<Size4KiB> =
        Page::range_inclusive(heap_start_page, heap_end_page);

    paging::phys::FRAME_ALLOCATOR
        .get()
        .map_range(
            heap_range,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        )
        .expect("Unable to map heap");

    info!(
        "{} Heap initialized at 0x{:x} - 0x{:x}",
        alloc_name, heap_start, heap_end
    );
    info!("Initializing {} allocator", alloc_name);
    unsafe { alloc_fn(heap_start.as_u64() as usize, heap_end.as_u64() as usize) };
    info!("{} allocator initialized", alloc_name);
}
