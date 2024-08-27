// :(.. this doesn't seem to work.
#![doc = include_str!("allocator-design-new.md")]

use core::alloc::GlobalAlloc;

use crate::util::OnceMutex;

use super::{block::Block, blocks::Blocks, LockedAllocator};

pub struct RuntimeAllocator {
    blocks: Blocks,
}

impl RuntimeAllocator {
    pub unsafe fn new(heap_start: usize, heap_end: usize) -> Self {
        Self {
            // SAFETY: Validity of the heap is guaranteed by the caller
            blocks: unsafe { Blocks::init(heap_start, heap_end) },
        }
    }
}

unsafe impl GlobalAlloc for LockedAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if layout.size() == 0 {
            // TODO: Figure out if this is the correct behavior for zero-sized allocations
            return core::ptr::null_mut();
        }
        let mut alloc = self.get();
        unsafe { alloc.blocks.allocate(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if layout.size() == 0 {
            return;
        }
        let mut alloc = self.get();
        unsafe { alloc.blocks.deallocate(ptr, layout) };
    }
}
