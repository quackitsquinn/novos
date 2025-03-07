use core::{
    alloc::Layout,
    fmt::Debug,
    mem, panic,
    ptr::{self},
};

use super::block::Block;
use crate::locked_vec::LockedVec;

pub struct BlockAllocator {
    // The blocks are stored in a downwards-growing vector.
    pub(crate) blocks: LockedVec<Block>,
    pub(crate) table_block: &'static mut Block,
    pub(crate) heap_start: usize,
    pub(crate) heap_end: usize,
    pub(crate) heap_size: usize,
    pub(crate) allocation_balance: isize,
}

unsafe impl Send for BlockAllocator {}
unsafe impl Sync for BlockAllocator {}

/// Count of blocks that can be allocated in the initial block table.
pub const INIT_BLOCK_SIZE: usize = 512;
/// The minimum heap size required for the block allocator.
pub const MIN_HEAP_SIZE: usize = INIT_BLOCK_SIZE * mem::size_of::<Block>();

impl BlockAllocator {
    pub unsafe fn init(heap_start: usize, heap_end: usize) -> Self {
        assert!(
            heap_end - heap_start > MIN_HEAP_SIZE,
            "Requested heap size too small, need at least {:#x} bytes",
            (INIT_BLOCK_SIZE * mem::size_of::<Block>()) + 512
        );
        let block_heap_end = (heap_end - mem::size_of::<Block>() as usize) as *mut Block;
        let block_table_base = unsafe { align(block_heap_end.sub(INIT_BLOCK_SIZE)) };
        // We already remove one block from the end of the heap, so we do not need to remove another if this is one/
        let off = block_table_base.1.saturating_sub(1);

        let (table_block, mut blocks) = unsafe {
            Self::init_block_table(block_table_base.0 as *mut Block, INIT_BLOCK_SIZE - off)
        };

        ainfo!("Block table base: {:p}", block_table_base.0);

        // Create a block that encompasses the entire heap.
        let block = Block::new(
            // Encompass all the heap except for the block table.
            (blocks.as_ptr() as usize - 1) - heap_start,
            heap_start as *mut u8,
            true,
        );

        blocks.push(block).expect("Failed to push block");

        Self {
            blocks,
            table_block,
            heap_start,
            heap_end,
            heap_size: heap_end - heap_start,
            allocation_balance: 0,
        }
    }

    unsafe fn init_block_table(
        ptr: *mut Block,
        table_size: usize,
    ) -> (&'static mut Block, LockedVec<Block>) {
        unsafe {
            let mut blocks = LockedVec::new(ptr, table_size);
            let table_block = Self::create_table_block(&mut blocks);
            (table_block, blocks)
        }
    }

    unsafe fn create_table_block(vec: &mut LockedVec<Block>) -> &'static mut Block {
        let block = Block::new(
            vec.capacity() * mem::size_of::<Block>(),
            vec.as_ptr() as *mut u8,
            false,
        );
        unsafe {
            vec.push_unchecked(block);
            &mut *vec.as_mut_ptr()
        }
    }

    unsafe fn push_block(&mut self, block: Block) {
        self.check_block_space();

        atrace!("Pushing block {:?}", block);
        self.blocks.push(block).expect("Failed to push block");
        self.condition_check();
    }

    /// Returns the block that `ptr` is allocated in.
    /// The returned block is not guaranteed to be allocated, so it is up to the caller to check if the block is allocated.
    pub fn find_block_by_ptr(&self, ptr: *mut u8) -> Option<&Block> {
        let ptr = ptr as usize;
        for block in &*self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size {
                atrace!(
                    "Found ptr ({:#x}) in block {:#p} (off: {})",
                    ptr,
                    block.address,
                    ptr - addr
                );
                return Some(block);
            }
        }
        None
    }

    unsafe fn find_block_by_ptr_mut(&mut self, ptr: *mut u8) -> Option<&mut Block> {
        let ptr = ptr as usize;
        if ptr < self.heap_start || ptr >= self.heap_end {
            return None;
        }
        for block in &mut *self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size {
                atrace!(
                    "Found ptr ({:#x}) in block {:#p} (off: {})",
                    ptr,
                    block.address,
                    ptr - addr
                );
                return Some(block);
            }
        }
        None
    }
    #[must_use = "Returned pointer must be deallocated with deallocate"]
    pub unsafe fn allocate(&mut self, layout: Layout) -> *mut u8 {
        // If the size is 0, we can just return a null pointer.
        if layout.size() == 0 {
            return ptr::null_mut();
        }
        let size = layout.size();
        self.condition_check();
        // If the alignment is 1, we don't need to align it.
        let alignment = if layout.align() == 1 {
            0
        } else {
            layout.align()
        };
        let mut split = None;
        let mut address: *mut u8 = ptr::null_mut();
        let full_size = size + alignment;

        if let Some(blk) = self.try_find_free_block(full_size) {
            ainfo!("Found free block {:?}", blk);
            address = blk.address;
            blk.allocate();
            if blk.size > full_size {
                split = blk.split(full_size);
            }
        } else {
            // If we didn't find a block, we need to run the GC and try again.
            self.gc();
            if let Some(blk) = self.try_find_free_block(full_size) {
                ainfo!("Found free block {:?}", blk);
                address = blk.address;
                blk.allocate();
                if blk.size > full_size {
                    split = blk.split(full_size);
                }
            }
        }

        // Because the whole heap is mapped, not finding a block either means that the heap became full or something went horribly wrong.
        if address.is_null() {
            aerror!("Failed to allocate block");
            return ptr::null_mut();
        }

        if let Some(block) = split {
            unsafe { self.push_block(block) };
        }

        self.allocation_balance += 1;

        let (ptr, off) = unsafe { align_ptr(address, layout.align()) };
        // This probably can't happen, but it's better to be safe than sorry.
        if off + size > full_size {
            panic!("Failed to align pointer");
        }

        atrace!("Returning pointer {:p}", ptr);
        self.condition_check();
        ptr
    }

    fn try_find_free_block(&mut self, size: usize) -> Option<&mut Block> {
        for block in &mut *self.blocks {
            if block.is_free && block.size >= size {
                return Some(block);
            }
        }
        None
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, _: Layout) -> Option<()> {
        self.condition_check();
        if let Some(blk) = unsafe { self.find_block_by_ptr_mut(ptr) } {
            ainfo!("Deallocating block {:?}", blk);
            if blk.is_free {
                panic!("Double free!");
            }
            blk.deallocate();
            ainfo!("Deallocated block {:?}", blk);
            self.allocation_balance -= 1;
            self.dbg_print_blocks();
            return Some(());
        }
        aerror!("Failed to find block for deallocation");

        #[allow(unreachable_code)]
        return None;
    }
    /// Run the block table garbage collector.
    pub fn gc(&mut self) {
        let mut last_free: Option<(usize, Block)> = None;
        let mut i = 0;
        let mut merge_total = 0;
        let mut merged = usize::MAX;
        while merged != 0 {
            merged = 0;
            while i < self.blocks.len() {
                let block = &mut self.blocks[i];
                if block.size == 0 {
                    aerror!("Found block with size 0: {:?}", block);
                    self.blocks.remove(i);
                    continue;
                }
                if block.is_free {
                    if let Some((idx, last_free_block)) = &mut last_free {
                        if last_free_block.is_adjacent(block) {
                            let new_block = last_free_block.merge(block);
                            self.blocks[*idx] = new_block;
                            self.blocks.remove(i);
                            merged += 1;

                            continue;
                        }
                    }
                    last_free = Some((i, block.clone()));
                }
                i += 1;
            }
            merge_total += merged;
            last_free = None;
            i = 0;
        }
        adebug!("Merged {} blocks", merge_total);
        self.condition_check();
    }

    /// Check if a pointer is allocated by the block allocator.
    #[inline]
    pub fn ptr_is_allocated(&self, ptr: *mut u8) -> bool {
        let ptr = ptr as usize;
        for block in &*self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size {
                return !block.is_free;
            }
        }
        false
    }

    fn check_block_space(&mut self) {
        if self.blocks.at_capacity() {
            self.gc();
        }

        if self.blocks.at_capacity() {
            panic!("Out of block space"); // TODO: This should try to reserve more space.
        }
    }

    fn dbg_print_blocks(&self) {
        let mx = self.table_block.size / size_of::<Block>();
        atrace!(
            "blkcount {}/{} ({}) umapbal: {}",
            self.blocks.len(),
            mx,
            self.blocks.len() as f64 / mx as f64,
            self.allocation_balance
        );

        let unalloc_count = self.blocks.iter().filter(|b| b.is_free).count();
        atrace!(
            "unallocs: {} allocs: {}",
            unalloc_count,
            self.blocks.len() - unalloc_count
        );
    }

    pub fn allocation_balance(&self) -> isize {
        self.allocation_balance
    }

    pub fn get_block_table<'a>(&'a self) -> &'a LockedVec<Block> {
        &self.blocks
    }
    // Get the block table mutably. This is unsafe because it allows the block table to be modified.
    pub unsafe fn get_block_table_mut<'a>(&'a mut self) -> &'a mut LockedVec<Block> {
        &mut self.blocks
    }

    pub fn table_block(&self) -> &Block {
        self.table_block
    }
    /// Gets the count of allocated blocks.
    /// Will always be >= 1 because the table block is always allocated.
    pub fn allocated_count(&self) -> usize {
        self.blocks.iter().filter(|b| !b.is_free).count()
    }
    /// Clear the block allocator. This function is ***INCREDIBLY UNSAFE*** and should only be used for testing.
    ///
    /// # Safety
    ///
    /// This function is so unsafe because it in essence calls mem::forget on anything allocated in the block allocator. This means that any pointers returned by the block allocator are now invalid and dereferencing them is undefined behavior.
    /// This function should only be used for testing purposes, and even then, it should be used with caution.
    #[cfg(test)]
    pub unsafe fn clear(&mut self) {
        // Zero the whole heap
        unsafe {
            ptr::write_bytes(
                self.heap_start as *mut u8,
                0,
                self.heap_end - (self.heap_start + 1),
            );
        }
        *self = unsafe { Self::init(self.heap_start, self.heap_end) };
    }
    /// Prints a debug representation of the block allocator.
    pub fn print_state(&self) {
        ainfo!("Start: {:#x}, End: {:#x}", self.heap_start, self.heap_end);
        ainfo!(
            "Allocated: {:#x}, Deallocated: {:#x}, Balance: {}",
            self.allocated_count(),
            self.blocks.len() - self.allocated_count(),
            self.allocation_balance,
        );
    }
    #[track_caller]
    pub fn condition_check(&self) {
        ainfo!("Checking block allocator condition");
        for block in &*self.blocks {
            if block.address >= self.heap_end as *mut u8
                || block.size == 0
                || block.size > self.heap_size as usize
                || block.address < self.heap_start as *mut u8
            {
                panic!("Found block with invalid address or size");
            }
        }
        ainfo!("Block allocator condition check passed");
    }
}

impl Debug for BlockAllocator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlockAllocator")
            .field("blocks", &self.blocks)
            .field("table_block", &self.table_block)
            .field("heap_start", &(self.heap_start as *const u8))
            .field("heap_end", &(self.heap_end as *const u8))
            .field("allocation_balance", &self.allocation_balance)
            .finish()
    }
}

/// Aligns a pointer to the specified alignment.
/// Returns a tuple containing the aligned pointer and the offset from the original pointer.
///
/// # Safety
///
/// This function is unsafe because it does not check if the pointer will overflow.
unsafe fn align_ptr(ptr: *mut u8, align: usize) -> (*mut u8, usize) {
    if ptr.addr() & (align - 1) == 0 {
        return (ptr, 0);
    }
    let offset = ptr.align_offset(align);
    (unsafe { ptr.add(offset) }, offset)
}

/// Aligns a pointer to T's alignment.
/// Returns a tuple containing the aligned pointer and the number of T's that the pointer was offset by.
/// The offset is not guaranteed to be a multiple of `size_of::<T>()`.
///
/// # Safety
///
/// This function is unsafe because it does not check if the pointer will overflow.
unsafe fn align<T>(ptr: *mut T) -> (*mut T, usize) {
    let align = mem::align_of::<T>();
    let (ptr, offset) = unsafe { align_ptr(ptr.cast(), align) };
    if offset == 0 {
        return (ptr.cast(), 0);
    }
    let offset = offset / mem::size_of::<T>();
    let is_multiple = offset % mem::size_of::<T>() == 0;
    (ptr.cast(), offset + if is_multiple { 0 } else { 1 })
}
