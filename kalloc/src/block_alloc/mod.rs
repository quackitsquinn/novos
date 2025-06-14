//! Kinda working block allocator
//!
//!
//! This is absolutely insane. Let me take this moment to scream.
//!
//! WHY DO THE TESTS FAIL ON MAC OS BUT NOT LINUX???? THIS ISN'T USING ANY PLATFORM DEPENDENT CODE
//! IT'S AN ALLOCATOR!!!! THE MEMORY BETWEEN THE TWO IS THE SAME!!! WHY DOES MAC OS RANDOMLY *RANDOMLY* FAIL??? FOR LIKE 4 DIFFERENT, INCONSISTENT REASONS????

use core::alloc::Allocator;
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
