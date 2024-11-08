use alloc::RuntimeAllocator;
use block::Block;
use downwards_vec::DownwardsVec;
use spin::MutexGuard;

use crate::{sprintln, util::OnceMutex};

pub mod alloc;
pub mod block;
pub mod blocks;
mod downwards_vec;

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

pub fn output_blocks() {
    sprintln!("{:#?}", ALLOCATOR.get().blocks);
}

pub fn get_allocation_balance() -> isize {
    ALLOCATOR.get().blocks.allocation_balance()
}

pub struct BlockLock<'a> {
    lock: MutexGuard<'a, RuntimeAllocator>,
}

impl<'a> BlockLock<'a> {
    pub fn new(lock: MutexGuard<'a, RuntimeAllocator>) -> Self {
        Self { lock }
    }

    pub fn get_block_table(&self) -> &DownwardsVec<Block> {
        self.lock.blocks.get_block_table()
    }
    /// Get the block table as mutable.
    ///
    /// # Safety
    /// The caller must ensure that the block table is not incorrectly modified.
    pub unsafe fn get_block_table_mut(&mut self) -> &'a DownwardsVec<Block> {
        unsafe { self.lock.blocks.get_block_table_mut() }
    }

    pub fn get_table_block(&self) -> &Block {
        self.lock.blocks.table_block()
    }
}

pub fn get_block_allocator() -> BlockLock<'static> {
    BlockLock::new(ALLOCATOR.get())
}
