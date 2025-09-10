use core::convert::Infallible;

use log::info;
use paging::map::{KERNEL_HEAP_SIZE, KERNEL_HEAP_START};
use x86_64::{
    structures::paging::{page::PageRangeInclusive, Page, PageTableFlags},
    VirtAddr,
};

use crate::{declare_module, memory::paging::KernelPageSize, requests::KERNEL_ELF};

pub mod allocator;
pub mod paging;
pub mod req_data;
pub mod stack;

pub const ALLOC_DEBUG: bool = option_env!("ALLOC_DEBUG").is_some();

declare_module!("memory", init);

fn init() -> Result<(), Infallible> {
    paging::MODULE.init();
    if ALLOC_DEBUG {
        kalloc::enable_logging();
    }
    init_heap();
    KERNEL_ELF.get().copy_to_heap();
    paging::vaddr_mapper::MODULE.init();
    paging::kernel::MODULE.init();
    Ok(())
}

fn init_heap() {
    configure_heap_allocator(
        "Kernel",
        allocator::init,
        KERNEL_HEAP_START,
        KERNEL_HEAP_SIZE,
    );
}

/// Configure a heap allocator with the given name, allocator function, and heap size.
/// alloc_fn should be a function that takes two usize arguments: the start and end of the heap (in that order).
fn configure_heap_allocator(
    alloc_name: &str,
    alloc_fn: unsafe fn(*mut u8, *mut u8),
    heap_start: VirtAddr,
    heap_size: u64,
) {
    let heap_end = heap_start + heap_size;
    let heap_start_page = Page::containing_address(heap_start);
    let heap_end_page = Page::containing_address(heap_end);
    let heap_range: PageRangeInclusive<KernelPageSize> =
        Page::range_inclusive(heap_start_page, heap_end_page);

    info!(
        "{} Heap range: 0x{:x} - 0x{:x} ({:?} - {:?}: {} pages)",
        alloc_name,
        heap_start,
        heap_end,
        heap_start_page,
        heap_end_page,
        heap_range.len()
    );

    unsafe {
        paging::phys::FRAME_ALLOCATOR.get().map_range(
            heap_range,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        )
    }
    .expect("Unable to map heap");

    info!(
        "{} Heap initialized at 0x{:x} - 0x{:x}",
        alloc_name, heap_start, heap_end
    );
    info!("Initializing {} allocator", alloc_name);
    unsafe { alloc_fn(heap_start.as_mut_ptr(), heap_end.as_mut_ptr()) };
    info!("{} allocator initialized", alloc_name);
}
