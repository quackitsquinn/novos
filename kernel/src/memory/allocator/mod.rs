use kalloc::{block_alloc::allocator::BlockAllocator, GlobalAllocatorWrapper};

#[global_allocator]
pub static ALLOCATOR: GlobalAllocatorWrapper<BlockAllocator> = GlobalAllocatorWrapper::new();

pub unsafe fn init(heap_start: *mut u8, heap_end: *mut u8) {
    //Command::InitIncrementalSend("alloc", "heap_snap{{ID}}.bin").send();
    kalloc::set_frame_output_fn(frame_output);
    ALLOCATOR.init(|| unsafe { BlockAllocator::init(heap_start.cast(), heap_end.cast(), true) });
}

pub fn frame_output(data: &[u8]) {
    //Command::SendIncrementalData("alloc", data).send();
}
