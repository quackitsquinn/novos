//! The global memory allocator.
use kalloc::{GlobalAllocatorWrapper, block_alloc::allocator::BlockAllocator};

/// The global memory allocator.
#[global_allocator]
pub static ALLOCATOR: GlobalAllocatorWrapper<BlockAllocator> = GlobalAllocatorWrapper::new();

pub(super) unsafe fn init(heap_start: *mut u8, heap_end: *mut u8) {
    ALLOCATOR.init(|| unsafe { BlockAllocator::init(heap_start.cast(), heap_end.cast(), false) });
}
