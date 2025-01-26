use core::{
    alloc::Layout,
    fmt::Debug,
    mem, panic,
    ptr::{self},
};

use kserial::common::Command;

use crate::{
    memory::allocator::log::{alloc_debug, alloc_error, alloc_info, alloc_trace},
    sprintln,
};

use super::{block::Block, locked_vec::LockedVec};

pub struct BlockAllocator {
    // The blocks are stored in a downwards-growing vector.
    blocks: LockedVec<Block>,
    table_block: &'static mut Block,
    heap_start: usize,
    heap_end: usize,
    allocation_balance: isize,
}

unsafe impl Send for BlockAllocator {}
unsafe impl Sync for BlockAllocator {}

// Count of blocks that can be allocated in the initial block table.
const INIT_BLOCK_SIZE: usize = 512;
// The amount of free space that must be left in a block after splitting.
const SPLIT_BYTE_INCREASE: usize = 8;
const GC_THRESHOLD: f64 = 0.8;

impl BlockAllocator {
    pub unsafe fn init(heap_start: usize, heap_end: usize) -> Self {
        let block_heap_end = (heap_end - mem::size_of::<Block>() as usize) as *mut Block;
        let block_table_base = unsafe { align(block_heap_end.sub(INIT_BLOCK_SIZE)) };
        // We already remove one block from the end of the heap, so we do not need to remove another if this is one/
        let off = block_table_base.1.saturating_sub(1);

        let (table_block, mut blocks) =
            unsafe { Self::init_table_at(block_table_base.0 as *mut Block, INIT_BLOCK_SIZE - off) };

        // Create a block that encompasses the entire heap.
        let block = Block::new(
            // Encompass all the heap except for the block table.
            blocks.as_ptr() as usize - heap_start,
            heap_start as *mut u8,
            true,
        );

        blocks.push(block).expect("Failed to push block");

        Self {
            blocks,
            table_block,
            heap_start,
            heap_end,
            allocation_balance: 0,
        }
    }

    unsafe fn init_table_at(
        ptr: *mut Block,
        table_size: usize,
    ) -> (&'static mut Block, LockedVec<Block>) {
        unsafe {
            let mut blocks = LockedVec::new(ptr, table_size);
            let table_block = Self::init_table_vec(&mut blocks);
            (table_block, blocks)
        }
    }

    unsafe fn init_table_vec(vec: &mut LockedVec<Block>) -> &'static mut Block {
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

        alloc_trace!("Pushing block {:?}", block);
        self.blocks.push(block).expect("Failed to push block");
    }

    /// Returns the block that `ptr` is allocated in.
    /// The returned block is not guaranteed to be allocated, so it is up to the caller to check if the block is allocated.
    pub fn find_block_by_ptr(&self, ptr: *mut u8) -> Option<&Block> {
        let ptr = ptr as usize;
        for block in &*self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size() {
                alloc_trace!(
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
        for block in &mut *self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size() {
                alloc_trace!(
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
        let size = layout.size();
        // If the alignment is 1, we don't need to align it.
        let alignment = if layout.align() == 1 {
            0
        } else {
            layout.align()
        };
        let mut split = None;
        let mut address: *mut u8 = ptr::null_mut();

        // If the size is 0, we can just return a null pointer.
        if size == 0 {
            return ptr::null_mut();
        }

        for block in &mut *self.blocks {
            if block.size == 0 {
                self.print_blocks_as_csv();
                panic!("Something has gone horribly wrong");
            }
            if block.is_free() && block.size() + alignment >= size {
                alloc_trace!("Found free block with correct size {:?}", block);
                block.allocate();
                address = block.address as *mut u8;
                if block.size() > size + SPLIT_BYTE_INCREASE + alignment {
                    alloc_trace!(
                        "Splitting block at {:?} at offset {}",
                        block,
                        size + alignment
                    );
                    split = block.split(size + alignment + SPLIT_BYTE_INCREASE);
                }
                break;
            }
        }

        // Because the whole heap is mapped, not finding a block either means that the heap became full or something went horribly wrong.
        if address.is_null() {
            alloc_error!("Failed to allocate block");
            return ptr::null_mut();
        }

        if let Some(block) = split {
            unsafe { self.push_block(block) };
        }

        self.allocation_balance += 1;

        // TODO: It might be a good idea to check if aligning the pointer goes out of bounds.
        let (ptr, _) = unsafe { align_ptr(address, layout.align()) };

        alloc_trace!("Returning pointer {:p}", ptr);
        ptr
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, _: Layout) -> Option<()> {
        if let Some(blk) = unsafe { self.find_block_by_ptr_mut(ptr) } {
            alloc_info!("Deallocating block {:?}", blk);
            if blk.is_free() {
                // TODO: Should DEFINITELY not panic here. Tis is a temporary debug check.
                alloc_error!("Block already deallocated");
                return None;
            }
            blk.deallocate();
            alloc_info!("Deallocated block {:?}", blk);
            self.allocation_balance -= 1;
            self.dbg_print_blocks();
            return Some(());
        }
        alloc_error!("Failed to find block for deallocation");

        #[allow(unreachable_code)]
        return None;
    }

    unsafe fn gc(&mut self) {
        let mut last_free: Option<(usize, Block)> = None;
        let mut i = 0;
        let mut merge_total = 0;
        let mut merged = usize::MAX;
        while merged != 0 {
            merged = 0;
            while i < self.blocks.len() {
                let block = &mut self.blocks[i];
                if block.size == 0 {
                    self.print_blocks_as_csv();
                    panic!("Something has gone horribly wrong");
                }
                if block.is_free() {
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
        alloc_debug!("Merged {} blocks", merge_total);
    }

    /// Check if a pointer is allocated by the block allocator.
    pub fn ptr_is_allocated(&self, ptr: *mut u8) -> bool {
        let ptr = ptr as usize;
        for block in &*self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size() {
                return !block.is_free();
            }
        }
        false
    }

    fn check_block_space(&mut self) {
        let size = self.table_block.size();
        // run GC if the block table is more than GC_THRESHOLD full
        if (self.blocks.len() * size_of::<Block>()) as f64 / size as f64 >= GC_THRESHOLD {
            unsafe { self.gc() };
        }

        if self.blocks.len() * mem::size_of::<Block>() >= size {
            self.dbg_serial_send_csv();
            panic!("Out of block space"); // TODO: This should try to reserve more space.
        }
    }

    fn dbg_print_blocks(&self) {
        let mx = self.table_block.size() / size_of::<Block>();
        alloc_trace!(
            "blkcount {}/{} ({}) umapbal: {}",
            self.blocks.len(),
            mx,
            self.blocks.len() as f64 / mx as f64,
            self.allocation_balance
        );

        let unalloc_count = self.blocks.iter().filter(|b| b.is_free()).count();
        alloc_trace!(
            "unallocs: {} allocs: {}",
            unalloc_count,
            self.blocks.len() - unalloc_count
        );
    }
    #[deprecated(note = "This function is named dumb. Use `print_blocks_as_csv` instead.")]
    fn dbg_serial_send_csv(&self) {
        #[cfg(debug_assertions)]
        {
            sprintln!("address,size,free");
            for block in &*self.blocks {
                sprintln!("{:p},{},{}", block.address, block.size(), block.is_free());
            }
        };
    }

    pub fn export_block_binary(&self, filename: &str) {
        // Unfortunately, we can't depend on the allocator (because if this is being called, the allocator is locked), so the only reliable way to send the blocks is to send them as the binary representation of the blocks.
        let mem_slice = &*self.blocks;
        let mem_slice_as_bytes = unsafe {
            core::slice::from_raw_parts(
                mem_slice.as_ptr() as *const u8,
                mem_slice.len() * mem::size_of::<Block>(),
            )
        };
        // TODO: Im not entirely confident that this is working correctly. I should probably test this.
        Command::SendFile(filename, mem_slice_as_bytes).send();
    }

    fn print_blocks_as_csv(&self) {
        sprintln!("address,size,free");
        for block in &*self.blocks {
            sprintln!("{:p},{},{}", block.address, block.size(), block.is_free());
        }
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
        self.blocks.iter().filter(|b| !b.is_free()).count()
    }
    /// Clear the block allocator. This function is ***INCREDIBLY UNSAFE*** and should only be used for testing.
    ///
    /// # Safety
    ///
    /// This function is so unsafe because it in essence calls mem::forget on anything allocated in the block allocator. This means that any pointers returned by the block allocator are now invalid and dereferencing them is undefined behavior.
    /// This function should only be used for testing purposes, and even then, it should be used with caution.
    #[cfg(test)]
    pub unsafe fn clear(&mut self) {
        *self = unsafe { Self::init(self.heap_start, self.heap_end) };
    }
    /// Prints a debug representation of the block allocator.
    pub fn print_state(&self) {
        sprintln!("Start: {:#x}, End: {:#x}", self.heap_start, self.heap_end);
        sprintln!(
            "Allocated: {:#x}, Deallocated: {:#x}, Balance: {}",
            self.allocated_count(),
            self.blocks.len() - self.allocated_count(),
            self.allocation_balance,
        );
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

mod tests {

    use alloc::{boxed::Box, vec::Vec};

    use crate::memory::allocator::{self, TEST_ALLOCATOR};

    use super::*;

    macro_rules! clear {
        () => {
            #[cfg(test)]
            unsafe {
                drop(TEST_ALLOCATOR.get().blocks.clear())
            }
        };
    }

    fn alloc_check<T>(ptr: *mut T, layout: Layout, allocator: &mut BlockAllocator) {
        assert!(!ptr.is_null());
        // Check if whole range is allocated
        for i in 0..layout.size() {
            assert!(
                allocator.ptr_is_allocated(unsafe { ptr.cast::<u8>().add(i).cast() }),
                "Failed at {}",
                i
            );
        }

        let block = allocator
            .find_block_by_ptr(ptr.cast())
            .expect("Allocated pointer not found");
        // Check if the block data is correct
        assert!(!block.is_free());
        assert!(block.size() >= layout.size());
        //        assert_eq!(block.address, ptr.cast());
        assert!(ptr.is_aligned())
    }

    #[kproc::test("Allocation", can_recover = true, bench_count = Some(100))]
    fn test_allocation() {
        let layout = Layout::from_size_align(512, 1).unwrap();

        let allocator = &mut TEST_ALLOCATOR.get().blocks;

        let ptr = unsafe { allocator.allocate(layout) };

        alloc_check(ptr, layout, allocator);

        unsafe { allocator.deallocate(ptr, layout) }.expect("Block failed to free");
        assert_eq!(allocator.allocation_balance, 0);
        assert!(!allocator.ptr_is_allocated(ptr));

        let block = allocator
            .find_block_by_ptr(ptr)
            .expect("Block pointer not found");

        assert!(block.size >= layout.size());
        assert!(block.is_free);
    }

    #[kproc::test("Block Join", can_recover = true)]
    fn test_block_join() {
        let layout = Layout::from_size_align(512, 1).unwrap();

        let allocator = &mut TEST_ALLOCATOR.get().blocks;

        let ptrs = [
            unsafe { allocator.allocate(layout) },
            unsafe { allocator.allocate(layout) },
            unsafe { allocator.allocate(layout) },
            unsafe { allocator.allocate(layout) },
        ];

        for ptr in &ptrs {
            alloc_check(*ptr, layout, allocator);
        }

        for ptr in &ptrs {
            unsafe { allocator.deallocate(*ptr, layout) }.expect("Block failed to free");
        }

        // Manually run GC
        unsafe { allocator.gc() };

        let block = allocator
            .find_block_by_ptr(ptrs[0])
            .expect("Block pointer not found");
        allocator.print_blocks_as_csv();
        assert!(block.size >= layout.size() * 4);
        assert!(block.is_free);
    }

    #[kproc::test("Block Reuse", can_recover = true)]
    fn test_block_reuse() {
        let layout = Layout::from_size_align(512, 1).unwrap();

        let allocator = &mut TEST_ALLOCATOR.get().blocks;

        let ptr = unsafe { allocator.allocate(layout) };

        unsafe {
            allocator
                .deallocate(ptr, layout)
                .expect("Block failed to free");
        }

        let new_ptr = unsafe { allocator.allocate(layout) };
        allocator.print_blocks_as_csv();
        assert_eq!(ptr, new_ptr);
    }

    #[kproc::test("Block Reuse w/ splitting block")]
    fn test_block_reuse_split() {
        let layout = Layout::from_size_align(512, 1).unwrap();

        let allocator = &mut TEST_ALLOCATOR.get().blocks;

        let ptrs = [
            unsafe { allocator.allocate(layout) },
            unsafe { allocator.allocate(layout) },
            unsafe { allocator.allocate(layout) },
            unsafe { allocator.allocate(layout) },
        ];

        unsafe {
            allocator
                .deallocate(ptrs[1], layout)
                .expect("Block failed to free");
            allocator
                .deallocate(ptrs[2], layout)
                .expect("Block failed to free");
        };

        unsafe { allocator.gc() };

        alloc_check(ptrs[0], layout, allocator);
        alloc_check(ptrs[3], layout, allocator);

        let dealloc_block_1 = allocator
            .find_block_by_ptr(ptrs[1])
            .expect("Unable to find ptr block");
        let dealloc_block_2 = allocator
            .find_block_by_ptr(ptrs[2])
            .expect("Unable to find ptr block");

        assert!(
            dealloc_block_1 == dealloc_block_2,
            "{:#?} != {:#?}",
            dealloc_block_1,
            dealloc_block_2
        )
    }

    #[kproc::test("Allocation with correct alignment")]
    fn test_alignment() {
        for i in 1..=12 {
            let layout = Layout::from_size_align(1, 1 << i).unwrap();

            let allocator = &mut TEST_ALLOCATOR.get().blocks;

            let ptr = unsafe { allocator.allocate(layout) };

            alloc_check(ptr, layout, allocator);
            assert!(ptr.is_aligned_to(1 << i));

            unsafe {
                allocator
                    .deallocate(ptr, layout)
                    .expect("Block failed to free");
            }
        }
    }

    #[kproc::test("Attempt to allocate ZST")]
    fn test_zst() {
        let layout = Layout::from_size_align(0, 1).unwrap();

        let allocator = &mut TEST_ALLOCATOR.get().blocks;

        let ptr = unsafe { allocator.allocate(layout) };

        assert!(ptr.is_null());
    }

    #[kproc::test("Box allocation", bench_count = Some(200))]
    fn test_box() {
        let value = 32u32;

        let bx = Box::new_in(value, &TEST_ALLOCATOR);
        let ptr = Box::into_raw(bx);

        let mut alloc = TEST_ALLOCATOR.get();
        let blocks = &mut alloc.blocks;

        assert_eq!(blocks.allocation_balance, 1);
        assert_eq!(unsafe { *ptr }, value);
        alloc_check(ptr, Layout::from_size_align(4, 1).unwrap(), blocks);
        drop(alloc);
        drop(unsafe { Box::from_raw_in(ptr, &TEST_ALLOCATOR) });
    }

    #[kproc::test("Vec allocation")]
    fn test_vec() {
        for i in 0..2000 {
            let mut vec: Vec<u32, &crate::util::OnceMutex<allocator::RuntimeAllocator>> =
                Vec::new_in(&TEST_ALLOCATOR);
            let layout = Layout::array::<u8>(100).expect("Failed to create layout");

            for i in 0..1000 {
                vec.push(i);
            }

            let ptr = vec.as_ptr().cast_mut();

            let mut alloc = TEST_ALLOCATOR.get();
            let blocks = &mut alloc.blocks;

            assert_eq!(blocks.allocation_balance, 1);
            assert_eq!(unsafe { *ptr }, 0);
            alloc_check(ptr, layout, blocks);
            drop(alloc);
            drop(vec);
            let mut alloc = &mut TEST_ALLOCATOR.get().blocks;
            assert!(!alloc.ptr_is_allocated(ptr as *mut u8));
            assert_eq!(alloc.allocation_balance, 0);
        }
        TEST_ALLOCATOR.get().blocks.print_blocks_as_csv();
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
