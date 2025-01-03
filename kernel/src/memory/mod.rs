use alloc::format;
use kserial::common::Command;
use limine::{memory_map::EntryType, paging::Mode, response::MemoryMapResponse};
use log::{info, trace};
use paging::{FRAME_ALLOCATOR, MEMORY_OFFSET, OFFSET_PAGE_TABLE};
use spin::Once;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper::MapperFlush, page::PageRangeInclusive, FrameAllocator, Mapper, OffsetPageTable,
        Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::{sprintln, util::OnceMutex};

pub mod allocator;
pub mod paging;

// Evaluates to 0x4156_4F4E_0000
pub const HEAP_MEM_OFFSET: u64 = (u32::from_ne_bytes(*b"NOVA") as u64) << 16;
pub const HEAP_SIZE: u64 = 1024 * 512; // 512 KiB

pub const TEST_HEAP_MEM_OFFSET: u64 = (u32::from_ne_bytes(*b"TEST") as u64) << 16;
pub const TEST_HEAP_SIZE: u64 = HEAP_SIZE; // 512 KiB

pub const MISC_MEM_OFFSET: u64 = (u32::from_ne_bytes(*b"MISC") as u64) << 16;

pub fn init() {
    paging::init();
    init_heap();
    paging::phys::init();
}

fn init_heap() {
    configure_heap_allocator("Kernel", allocator::init, HEAP_MEM_OFFSET, HEAP_SIZE);
    configure_heap_allocator(
        "Kernel Test",
        allocator::init_test,
        TEST_HEAP_MEM_OFFSET,
        TEST_HEAP_SIZE,
    );
}

/// Configure a heap allocator with the given name, allocator function, and heap size.
/// alloc_fn should be a function that takes two usize arguments: the start and end of the heap (in that order).
fn configure_heap_allocator(
    alloc_name: &str,
    alloc_fn: unsafe fn(usize, usize),
    heap_offset: u64,
    heap_size: u64,
) {
    let heap_start = VirtAddr::new(heap_offset);
    let heap_end = heap_start + heap_size - 1u64;
    let heap_start_page = Page::containing_address(heap_start);
    let heap_end_page = Page::containing_address(heap_end);
    let heap_range: PageRangeInclusive<Size4KiB> =
        Page::range_inclusive(heap_start_page, heap_end_page);

    let mut pfa = FRAME_ALLOCATOR.get();
    for page in heap_range {
        pfa.map_page(page, PageTableFlags::PRESENT | PageTableFlags::WRITABLE)
            .unwrap()
            .flush();
    }

    info!(
        "{} Heap initialized at 0x{:x} - 0x{:x}",
        alloc_name, heap_start, heap_end
    );
    info!("Initializing {} allocator", alloc_name);
    unsafe { alloc_fn(heap_start.as_u64() as usize, heap_end.as_u64() as usize) };
    info!("{} allocator initialized", alloc_name);
}

pub unsafe fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    let offset = MEMORY_OFFSET.get().expect("Memory offset not set");
    trace!("Adding {:x} to {:x}", offset, phys.as_u64());
    VirtAddr::new(phys.as_u64() + MEMORY_OFFSET.get().expect("Memory offset not set"))
}
