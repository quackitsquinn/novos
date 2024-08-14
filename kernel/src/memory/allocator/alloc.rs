// :(.. this doesn't seem to work.
#![doc = include_str!("allocation.md")]

use core::alloc::GlobalAlloc;

use crate::util::OnceMutex;

use super::block::Block;

#[global_allocator]
static ALLOCATOR: LockedAllocator = LockedAllocator::new();

type LockedAllocator = OnceMutex<RuntimeAllocator>;

pub struct RuntimeAllocator {
    heap_start: usize,
    heap_end: usize,
    /// An array of blocks, each representing a block of memory. Grows downwards from the end of the heap.
    blocks: &'static mut [Option<Block>],
}

impl RuntimeAllocator {
    pub unsafe fn new(heap_start: usize, heap_end: usize) -> Self {
        todo!()
    }
}

unsafe impl GlobalAlloc for RuntimeAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}

unsafe impl GlobalAlloc for LockedAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if layout.size() == 0 {
            // TODO: Figure out if this is the correct behavior for zero-sized allocations
            return core::ptr::null_mut();
        }
        unsafe { self.get().alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if layout.size() == 0 {
            return;
        }
        unsafe { self.get().dealloc(ptr, layout) }
    }
}
