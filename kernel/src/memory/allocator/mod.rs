#![doc = include_str!("allocator-design-new.md")]

use core::{
    alloc::{AllocError, Allocator, GlobalAlloc},
    ptr::NonNull,
};

use block_alloc::BlockAllocator;

use crate::util::OnceMutex;

pub mod block;
pub mod block_alloc;
mod locked_vec;
mod log;

pub struct RuntimeAllocator {
    pub(crate) blocks: BlockAllocator,
}

impl RuntimeAllocator {
    pub unsafe fn new(heap_start: usize, heap_end: usize) -> Self {
        Self {
            // SAFETY: Validity of the heap is guaranteed by the caller
            blocks: unsafe { BlockAllocator::init(heap_start, heap_end) },
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
        // if layout.size() == 0 {
        //     return;
        // }
        let mut alloc = self.get();
        unsafe { alloc.blocks.deallocate(ptr, layout) };
    }
}

unsafe impl Allocator for LockedAllocator {
    fn allocate(&self, layout: core::alloc::Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() == 0 {
            return Ok(NonNull::slice_from_raw_parts(NonNull::dangling(), 0));
        }
        let ptr = unsafe { self.get().blocks.allocate(layout) };
        if ptr.is_null() {
            Err(AllocError)
        } else {
            Ok(NonNull::slice_from_raw_parts(
                NonNull::new(ptr).ok_or(AllocError)?,
                layout.size(),
            ))
        }
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        unsafe { self.get().blocks.deallocate(ptr.as_ptr(), layout) };
    }
}

#[global_allocator]
pub static ALLOCATOR: LockedAllocator = LockedAllocator::new();

/// An allocator purely for testing. This allocator is reset after every test, so it is unsafe to store any static variables in this allocator.
pub static TEST_ALLOCATOR: LockedAllocator = LockedAllocator::new();

pub type LockedAllocator = OnceMutex<RuntimeAllocator>;

pub unsafe fn init(heap_start: usize, heap_end: usize) {
    ALLOCATOR.init(unsafe { RuntimeAllocator::new(heap_start, heap_end) });
}

pub unsafe fn init_test(heap_start: usize, heap_end: usize) {
    TEST_ALLOCATOR.init(unsafe { RuntimeAllocator::new(heap_start, heap_end) });
}
