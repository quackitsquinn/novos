//! Block based allocator.

use core::alloc::Layout;

use allocator::BlockAllocator;

use crate::mut_alloc::MutableAllocator;
#[cfg(test)]
mod alloc_tests;
pub mod allocator;
pub mod block;

unsafe impl MutableAllocator for BlockAllocator {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        unsafe { self.allocate(layout) }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.deallocate(ptr, layout);
        }
    }
}
