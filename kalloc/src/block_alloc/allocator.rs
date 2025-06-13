// TODO: Strict Provenance? Would that even be possible?
use core::{
    alloc::Layout,
    fmt::Debug,
    mem,
    ptr::{self},
};

use super::block::Block;
use crate::{frame_output, locked_vec::LockedVec};

pub struct BlockAllocator {
    // The blocks are stored in a downwards-growing vector.
    pub(crate) blocks: LockedVec<Block>,
    pub(crate) table_block: &'static mut Block,
    pub(crate) heap_start: usize,
    pub(crate) heap_end: usize,
    pub(crate) heap_size: usize,
    pub(crate) allocation_balance: isize,
}

/// Count of blocks that can be allocated in the initial block table.
pub const INIT_BLOCK_SIZE: usize = 512;
/// The minimum heap size required for the block allocator.
pub const MIN_HEAP_SIZE: usize = INIT_BLOCK_SIZE * mem::size_of::<Block>();

impl BlockAllocator {
    /// Initializes the block allocator.
    ///
    /// `write_uninit` is used to determine if the heap should be fully written with `0x0F` (or 0x00f) or not.
    ///
    /// # Safety
    ///
    /// - `heap_start` and `heap_end` must be aligned to 8 bytes and are inclusive.
    /// - `heap_start` must be less than `heap_end`.
    /// - The heap must be at least `MIN_HEAP_SIZE` bytes.
    /// - Writes and reads through `heap_start` and `heap_end` must be valid.
    pub unsafe fn init(heap_start: *mut u64, heap_end: *mut u64, write_uninit: bool) -> Self {
        // Create a bunch of useful variables.
        let heap_start_usize = heap_start as usize;
        let heap_end_usize = heap_end as usize;
        let heap_start = heap_start as *mut u8;
        let heap_end = heap_end as *mut u8;
        let heap_size = heap_end_usize - heap_start_usize;
        // Precondition checks.
        assert!(
            heap_size > MIN_HEAP_SIZE,
            "Requested heap size too small, need at least {:#x} bytes",
            MIN_HEAP_SIZE
        );
        assert!(
            heap_start_usize < heap_end_usize,
            "Heap start must be less than heap end"
        );
        assert!(
            // TODO: Maybe a higher alignment is needed? Might be a good idea to have the alignment be 4k. (Page size)
            heap_start.is_aligned() && heap_end.is_aligned(),
            "Heap start and end must be aligned"
        );

        if write_uninit {
            unsafe {
                ptr::write_bytes(heap_start, 0x0F, heap_size);
            }
        }

        // Create the base for the block table
        let (block_table_base, applied_offset) =
            unsafe { align(heap_end.cast::<Block>().sub(INIT_BLOCK_SIZE)) };

        // If the offset is not 0, we need to subtract 1 from the block table size.
        let off = if applied_offset == 0 { 0 } else { 1 };

        let (table_block, mut blocks) =
            unsafe { Self::create_block_table(block_table_base, INIT_BLOCK_SIZE - off) };

        ainfo!("Block table base: {:p}", block_table_base);

        // Create a block that encompasses the entire heap.
        let block = Block::new(
            // Encompass all the heap except for the block table.
            heap_size - blocks.byte_size(),
            heap_start_usize as *mut u8,
            true,
        );

        blocks.push(block).expect("Failed to push block");

        Self {
            blocks,
            table_block,
            heap_start: heap_start_usize,
            heap_end: heap_end_usize,
            heap_size,
            allocation_balance: 0,
        }
    }
    /// Creates a block table at the specified pointer.
    unsafe fn create_block_table(
        ptr: *mut Block,
        table_size: usize,
    ) -> (&'static mut Block, LockedVec<Block>) {
        unsafe {
            let mut blocks = LockedVec::new(ptr, table_size);
            let block = Block::new(blocks.byte_size(), blocks.as_mut_ptr().cast(), false);
            blocks.push(block).expect("Failed to push block");
            (&mut *blocks.as_mut_ptr(), blocks)
        }
    }
    /// Pushes a block to the block table.
    unsafe fn push_block(&mut self, block: Block) {
        self.check_block_space();

        atrace!("Pushing block {:?}", block);
        self.blocks.push(block).expect("Failed to push block");
        self.condition_check();
    }
    /// This is a helper function that finds the block that `ptr` is allocated in.
    ///
    /// This function itself is not unsafe, but how the returned value is used can be unsafe.
    ///
    /// The return value is guaranteed to be a valid block if it is not `None`.
    fn find_block_by_ptr_internal(&self, ptr: *mut u8) -> Option<usize> {
        let ptr = ptr as usize;
        for block in &*self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size {
                return Some(block as *const Block as usize);
            }
        }
        None
    }

    /// Returns the block that `ptr` is allocated in.
    /// This will return the block that contains `ptr` even if it is not allocated.
    pub fn find_block_by_ptr(&self, ptr: *mut u8) -> Option<&Block> {
        if let Some(ptr) = self.find_block_by_ptr_internal(ptr) {
            return Some(unsafe { &*(ptr as *const Block) });
        }
        None
    }

    /// Returns the block that `ptr` is allocated in.
    /// This will return the block that contains `ptr` even if it is not allocated.
    ///
    /// # Safety
    /// This function is *incredibly* unsafe and any modifications to the block should not be taken lightly.
    /// Virtually almost all modifications can result in undefined behavior.
    pub unsafe fn find_block_by_ptr_mut(&mut self, ptr: *mut u8) -> Option<&mut Block> {
        if let Some(ptr) = self.find_block_by_ptr_internal(ptr) {
            return Some(unsafe { &mut *(ptr as *mut Block) });
        }
        None
    }

    /// Allocates a block of memory with the specified layout.
    ///
    /// # Safety
    ///
    /// The returned pointer must be deallocated with `deallocate` to prevent memory leaks.
    #[must_use = "Returned pointer must be deallocated with deallocate"]
    pub unsafe fn allocate(&mut self, layout: Layout) -> *mut u8 {
        frame_output(unsafe {
            core::slice::from_raw_parts(self.heap_start as *mut u8, self.heap_size)
        });
        // If the size is 0, we can just return a null pointer.
        if layout.size() == 0 {
            return ptr::null_mut();
        }
        let size = layout.size();

        // If the alignment is 1, we don't need to align it.
        let alignment = if layout.align() == 1 {
            0
        } else {
            layout.align()
        };
        let mut split = None;
        let mut address: *mut u8 = ptr::null_mut();
        let full_size = size + alignment;
        let mut block_size = 0;

        // Loop twice to try to find a free block.
        for _ in 0..2 {
            if let Some(blk) = self.try_find_free_block(full_size) {
                ainfo!("Found free block {:?}", blk);
                address = blk.address;
                block_size = blk.size;
                blk.allocate();
                if blk.size > full_size {
                    let rem = blk.split(full_size);
                    if rem.is_none() {
                        aerror!("Failed to split block");
                        return ptr::null_mut();
                    }
                    split = rem;
                }
                break;
            }
            self.defrag();
        }

        // Because the whole heap is mapped, not finding a block either means that the heap became full or something went horribly wrong.
        if address.is_null() {
            aerror!("Failed to allocate block");
            return ptr::null_mut();
        }

        if (address as usize + block_size) > self.blocks.as_mut_ptr() as usize {
            panic!(
                "Block address is out of bounds: {:#x} > {:#x}",
                address as usize + size,
                self.blocks.as_mut_ptr() as usize
            );
        }

        if let Some(block) = split {
            for blk in &*self.blocks {
                if blk.contains(&block) {
                    panic!(
                        "Memory Corruption: Block {:?} contains split block {:?}",
                        blk, block
                    );
                }
            }
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
        frame_output(unsafe {
            core::slice::from_raw_parts(self.heap_start as *mut u8, self.heap_size)
        });
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

    /// Deallocates a block of memory.
    ///
    /// # Safety
    /// The pointer must have been allocated by the block allocator.
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
            self.print_state();
            return Some(());
        }

        aerror!("Failed to find block for deallocation");
        return None;
    }
    /// Defragments the block allocator.
    #[inline]
    pub fn defrag(&mut self) {
        let mut merge_total = 0;
        let mut merged = usize::MAX;
        while merged > 0 {
            merged = self.defragment_single_pass();
            merge_total += merged;
            atrace!("GC: Table: {:?}", self.blocks);
        }
        adebug!("Merged {} blocks", merge_total);
        self.condition_check();
    }

    /// Runs a single pass of the defragmentation algorithm. Returns the number of blocks merged.\
    #[inline]
    fn defragment_single_pass(&mut self) -> usize {
        let mut i = 0;
        let mut merged = 0;
        let mut last_free: Option<(usize, Block)> = None;
        if option_env!("DEFRAG_PRE_DEBUG").is_some() {
            atrace!("DEFRAG: Table Before: {:?}", self.blocks);
        }
        // We do this while loop because the length of the blocks vector can change.
        while i < self.blocks.len() {
            let block = &mut self.blocks[i];
            // Sanity check
            debug_assert!(block.size > 0, "Found block with invalid size: {:?}", block);

            // If the block is not free, we can skip it.
            if !block.is_free {
                atrace!("DEFRAG: Skipping allocated block {:?}", block);
                i += 1;
                continue;
            }

            // If we haven't found a free block yet, we can just set it and continue.
            if last_free.is_none() {
                last_free = Some((i, block.clone()));
                atrace!("DEFRAG: Found free block {:?}", block);
                i += 1;
                continue;
            }

            // We know that last_free is not None, so we can unwrap it.
            let (idx, last_free_block) = &mut last_free.as_mut().unwrap();

            // If the blocks are not adjacent, we can just set the last free block to the current block and continue.
            if !last_free_block.is_adjacent(block) {
                atrace!("DEFRAG: Found non-adjacent free block {:?}", block);
                i += 1;
                *last_free_block = block.clone();
                *idx = i;
                continue;
            }

            let new_block = last_free_block.merge(block);
            atrace!(
                "DEFRAG: Merged blocks {:?} and {:?}",
                last_free_block,
                block
            );
            self.blocks[*idx] = new_block.clone();

            // This can't be inlined into `atrace!` because it is not guaranteed to be evaluated.
            let removed_block = self.blocks.remove(i);

            atrace!(
                "DEFRAG: New block[{}] {:?}; Removing block[{}] {:?}",
                *idx,
                new_block,
                i,
                removed_block,
            );
            merged += 1;
            last_free = Some((*idx, new_block));
        }
        merged
    }

    /// Check if a pointer is allocated by the block allocator.
    #[inline]
    pub fn ptr_is_allocated(&self, ptr: *mut u8) -> bool {
        self.find_block_by_ptr(ptr)
            .map(|b| !b.is_free)
            .unwrap_or(false)
    }

    /// Check if the block allocator has enough space to allocate a block. If not, it will defragment the block allocator.
    /// If the block allocator is still full, it will panic.
    fn check_block_space(&mut self) {
        if self.blocks.at_capacity() {
            self.defrag();
        }

        if self.blocks.at_capacity() {
            panic!("Out of block space"); // TODO: This should try to reserve more space.
        }
    }

    /// Print debug information about the blocks.
    pub fn print_state(&self) {
        let allocated = self.allocated_count();
        let unallocated = self.blocks.len() - allocated;
        atrace!(
            "Allocated / Deallocated: {}/{}; Total/Capacity: {}/{}; Balance: {}; {:#x} - {:#x}",
            allocated,
            unallocated,
            self.blocks.len(),
            self.blocks.capacity(),
            self.allocation_balance,
            self.heap_start,
            self.heap_end
        );
    }

    /// Get the allocation balance.
    pub fn allocation_balance(&self) -> isize {
        self.allocation_balance
    }
    /// Get the block table.
    pub fn get_block_table<'a>(&'a self) -> &'a LockedVec<Block> {
        &self.blocks
    }

    /// Get the block table mutably.
    ///
    /// This function is incredibly unsafe. **Only** use this function if you know what you are doing.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it allows for mutable access to the block table. This can lead to undefined behavior if used incorrectly.
    pub unsafe fn get_block_table_mut<'a>(&'a mut self) -> &'a mut LockedVec<Block> {
        &mut self.blocks
    }

    /// Get the block table.
    pub fn table_block(&self) -> &Block {
        self.table_block
    }

    /// Gets the count of allocated blocks.
    /// Will always be >= 1 because the table block is always allocated.
    pub fn allocated_count(&self) -> usize {
        self.blocks.iter().filter(|b| !b.is_free).count()
    }

    /// Returns whether the block allocator has leaked memory. This is essentially a check to see if the allocation balance is 0.
    ///
    /// This can be a useful heuristic during testing to see if the block allocator has leaked memory.
    pub fn did_leak(&self) -> bool {
        self.allocation_balance != 0
    }

    /// Clear the block allocator. This function is ***INCREDIBLY UNSAFE*** and should only be used for testing.
    ///
    /// # Safety
    ///
    /// This function is so unsafe because it in essence calls mem::forget on anything allocated in the block allocator. This means that any pointers returned by the block allocator are now invalid and dereferencing them is undefined behavior.
    /// This function should only be used for testing purposes, and even then, it should be used with caution.
    #[cfg(test)]
    pub unsafe fn clear(&mut self, write_uninit: bool) {
        // Zero the whole heap
        unsafe {
            ptr::write_bytes(
                self.heap_start as *mut u8,
                0,
                self.heap_end - (self.heap_start + 1),
            );
        }
        *self = unsafe {
            Self::init(
                self.heap_start as *mut u64,
                self.heap_end as *mut u64,
                write_uninit,
            )
        };
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
                panic!("Found block with invalid address or size: {:?}", block);
            }
        }
        ainfo!("Block allocator condition check passed");
    }

    pub fn heap_snapshot(&self) {
        let slc =
            unsafe { core::slice::from_raw_parts(self.heap_start as *const u8, self.heap_size) };
        crate::frame_output(slc);
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
pub(crate) unsafe fn align_ptr<T>(ptr: *mut T, align: usize) -> (*mut T, usize) {
    if !align.is_power_of_two() {
        panic!("Alignment is not a power of two");
    }
    // Pointer is already aligned.
    // SAFETY: We know that `align` is a power of two, and so it must be non-zero.
    if ptr.addr() & (unsafe { align.unchecked_sub(1) }) == 0 {
        return (ptr, 0);
    }
    let offset = ptr.cast::<u8>().align_offset(align);
    (unsafe { ptr.cast::<u8>().add(offset).cast() }, offset)
}

/// Aligns a pointer to T's alignment.
/// Returns a tuple containing the aligned pointer and the number of T's that the pointer was offset by.
/// The offset is not guaranteed to be a multiple of `size_of::<T>()`.
///
/// # Safety
///
/// This function is unsafe because it does not check if the pointer will overflow.
unsafe fn align<T>(ptr: *mut T) -> (*mut T, usize) {
    unsafe { align_ptr(ptr, mem::align_of::<T>()) }
}
