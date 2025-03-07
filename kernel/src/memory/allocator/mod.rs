use kalloc::{block_alloc::allocator::BlockAllocator, GlobalAllocatorWrapper};

#[global_allocator]
pub static ALLOCATOR: GlobalAllocatorWrapper<BlockAllocator> = GlobalAllocatorWrapper::new();

pub unsafe fn init(heap_start: usize, heap_end: usize) {
    ALLOCATOR.init(|| unsafe { BlockAllocator::init(heap_start, heap_end) });
}
