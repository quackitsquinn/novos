use alloc::RuntimeAllocator;

use crate::{sprint, sprintln, util::OnceMutex};

pub mod alloc;
pub mod block;
pub mod blocks;
mod downwards_vec;

#[global_allocator]
pub static ALLOCATOR: LockedAllocator = LockedAllocator::new();

pub type LockedAllocator = OnceMutex<RuntimeAllocator>;

pub unsafe fn init(heap_start: usize, heap_end: usize) {
    ALLOCATOR.init(unsafe { RuntimeAllocator::new(heap_start, heap_end) });
}

pub fn output_blocks() {
    sprintln!("{:#?}", ALLOCATOR.get().blocks);
}

pub fn get_allocation_balance() -> isize {
    ALLOCATOR.get().blocks.allocation_balance()
}
