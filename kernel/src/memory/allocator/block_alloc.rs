use core::{
    alloc::Layout,
    fmt::Debug,
    mem,
    ptr::{self},
};

use log::{debug, error, info, trace};

use crate::{debug_release_select, sprintln};

use super::{block::Block, downwards_vec::DownwardsVec};

pub struct BlockAllocator {
    // The blocks are stored in a downwards-growing vector.
    blocks: DownwardsVec<'static, Block>,
    table_block: &'static mut Block,
    unmap_start: usize, // The start of the unmapped memory.
    heap_start: usize,
    heap_end: usize,
    allocation_balance: isize,
}

unsafe impl Send for BlockAllocator {}
unsafe impl Sync for BlockAllocator {}

// Count of blocks that can be allocated in the inital block table.
const INIT_BLOCK_SIZE: usize = 512;
const BLOCK_TABLE_BLOCK_SIZE: usize = 128;
const BLOCK_SIZE_BYTES: usize = mem::size_of::<Block>() * INIT_BLOCK_SIZE;
const SPLIT_THRESHOLD: f64 = 0.5;
const GC_THRESHOLD: f64 = 0.8;

impl BlockAllocator {
    pub unsafe fn init(heap_start: usize, heap_end: usize) -> Self {
        let block_heap_end = heap_end - mem::size_of::<Block>();
        let block_table_base = align(block_heap_end as *mut Block, true) as usize;

        let (table_block, blocks) =
            unsafe { Self::init_table_at(block_table_base as *mut Block, INIT_BLOCK_SIZE) };

        Self {
            blocks,
            table_block,
            heap_start,
            heap_end,
            unmap_start: heap_start,
            allocation_balance: 0,
        }
    }
    /// Creates a new block allocator with the given heap start and end. Only configured for testing because I can't think of a real world use case for this.
    //#[cfg(test)]
    pub(crate) unsafe fn init_at(
        heap_start: usize,
        heap_end: usize,
        block_table_base: *mut Block,
        block_table_size: usize,
    ) -> Self {
        let (table_block, blocks) =
            unsafe { Self::init_table_at(block_table_base as *mut Block, block_table_size) };
        info!(
            "Creating block allocator with heap start: {:#x} and heap end: {:#x}",
            heap_start, heap_end
        );
        Self {
            blocks,
            table_block,
            heap_start,
            heap_end,
            unmap_start: heap_start,
            allocation_balance: 0,
        }
    }
    /// Creates a new block allocator with the given heap start, end, and block table.
    ///
    /// # Safety
    /// heap_start and heap_end must be valid memory addresses, and if not, any pointer returned
    /// by this allocator will be invalid and is undefined behavior to dereference.
    pub(crate) unsafe fn init_with_vec(
        heap_start: usize,
        heap_end: usize,
        mut blocks: DownwardsVec<'static, Block>,
    ) -> Self {
        let table_block = unsafe { Self::init_table_vec(&mut blocks) };
        Self {
            blocks,
            table_block,
            heap_start,
            heap_end,
            unmap_start: heap_start,
            allocation_balance: 0,
        }
    }

    unsafe fn init_table_at(
        ptr: *mut Block,
        table_size: usize,
    ) -> (&'static mut Block, DownwardsVec<'static, Block>) {
        unsafe {
            let mut blocks = DownwardsVec::new(ptr, table_size);
            let table_block = Self::init_table_vec(&mut blocks);
            (table_block, blocks)
        }
    }

    unsafe fn init_table_vec(vec: &mut DownwardsVec<'static, Block>) -> &'static mut Block {
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

        // TODO: Use the actual allocator design here. This is a temporary solution because I got tired of fighting with pointers.
        for blks in &mut *self.blocks {
            if blks.is_reusable {
                trace!("Reusing block {:?}", blks);
                *blks = block;
                return;
            }
        }

        trace!("Pushing block {:?}", block);
        self.blocks.push(block).expect("Failed to push block");
    }

    // This will be relatively slow, but it should be called less and less as the heap grows.
    unsafe fn allocate_block(&mut self, size: usize) -> Block {
        if self.unmap_start + size >= self.heap_end {
            panic!("Out of memory");
        }
        self.dbg_print_blocks();
        let block = Block::new(size, self.unmap_start as *mut u8, false);
        self.unmap_start += size;
        self.dbg_print_blocks();
        block
    }
    /// Returns the block that `ptr` is allocated in.
    /// The returned block is not guaranteed to be allocated, so it is up to the caller to check if the block is allocated.
    pub fn find_block_by_ptr(&self, ptr: *mut u8) -> Option<&Block> {
        let ptr = ptr as usize;
        for block in &*self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size() && !block.is_reusable {
                trace!(
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
                trace!(
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
        let mut address = ptr::null_mut();

        if size == 0 {
            return ptr::null_mut();
        }

        for block in &mut *self.blocks {
            if block.is_free() && block.size() + alignment >= size {
                trace!("Found free block with correct size {:?}", block);
                block.allocate();
                address = block.address as *mut u8;
                if block.size() > size + (size as f64 * SPLIT_THRESHOLD) as usize {
                    trace!(
                        "Splitting block at {:?} at offset {}",
                        block,
                        size + alignment
                    );
                    split = block.split(size + alignment);
                }
            }
        }

        if address.is_null() {
            trace!("No free block found; allocating new block");
            // Allocate a new block
            let block = unsafe { self.allocate_block(size + alignment) };
            address = block.address as *mut u8;
            unsafe { self.push_block(block) };
        }

        if let Some(block) = split {
            unsafe { self.push_block(block) };
        }

        self.allocation_balance += 1;

        let ptr = align_with_alignment(address, layout.align(), false);
        trace!("Returning pointer {:p}", ptr);
        ptr
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) -> Option<()> {
        if let Some(blk) = // OLD: align_with_alignment(ptr, layout.align(), true))
            unsafe { self.find_block_by_ptr_mut(ptr) }
        {
            info!("Deallocating block {:?}", blk);
            if blk.is_free() {
                // TODO: Should DEFINITELY not panic here. Tis is a temporary debug check.
                debug_release_select! {
                    debug {
                        error!("Block already deallocated");
                        self.dbg_serial_send_csv();
                        //panic!("Block already deallocated");
                    },
                    release {
                        error!("Block already deallocated");
                    }
                }
                return None;
            }
            blk.deallocate();
            info!("Deallocated block {:?}", blk);
            self.allocation_balance -= 1;
            self.dbg_print_blocks();
            return Some(());
        }
        debug_release_select!(
            debug {
                panic!("Block not found");
            },
            release {
                error!("Block not found");
            }
        );

        #[allow(unreachable_code)]
        return None;
    }

    unsafe fn gc(&mut self) {
        let mut last_free_block: Option<&mut Block> = None;
        let mut joined = 0;
        // Sort the blocks by address
        self.blocks.sort_by(|a, b| a.address.cmp(&b.address));
        for block in &mut *self.blocks {
            if block.is_free() {
                if let Some(lsblk) = last_free_block {
                    if !lsblk.is_adjacent(block) {
                        last_free_block = None;
                        continue;
                    }
                    info!("Joining blocks {:?} and {:?}", lsblk, block);
                    info!(
                        "Address: {:#x} Size: {}",
                        lsblk.address as usize,
                        lsblk.size()
                    );
                    info!(
                        "Address: {:#x} Size: {}",
                        block.address as usize,
                        block.size()
                    );
                    *block = block.merge(lsblk);
                    joined += 1;
                    lsblk.set_reusable(true);
                }
                last_free_block = Some(block)
            } else {
                last_free_block = None;
            }
        }
        debug!("Joined {} blocks", joined);
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
            self.run_gc();
        }

        if self.blocks.len() * mem::size_of::<Block>() >= size {
            self.dbg_serial_send_csv();
            panic!("Out of block space"); // TODO: This should try to reserve more space.
        }
    }

    fn run_gc(&mut self) {
        // sprintln!("START-PRE-GC");
        // self.dbg_serial_send_csv();
        // sprintln!("END-PRE-GC");
        unsafe {
            self.gc();
        }
        // sprintln!("START-POST-GC");
        // self.dbg_serial_send_csv();
        // sprintln!("END-POST-GC");
    }

    fn dbg_print_blocks(&self) {
        let mx = self.table_block.size() / size_of::<Block>();
        trace!(
            "blkcount {}/{} ({}) umapbal: {}",
            self.blocks.len(),
            mx,
            self.blocks.len() as f64 / mx as f64,
            self.allocation_balance
        );

        let unalloc_count = self.blocks.iter().filter(|b| b.is_free()).count();
        trace!(
            "unallocs: {} allocs: {}",
            unalloc_count,
            self.blocks.len() - unalloc_count
        );
    }

    fn dbg_serial_send_csv(&self) {
        #[cfg(debug_assertions)]
        {
            sprintln!("address,size,free,can_reuse");
            for block in &*self.blocks {
                sprintln!(
                    "{:p},{},{},{}",
                    block.address,
                    block.size(),
                    block.is_free(),
                    block.is_reusable
                );
            }
        };
    }

    pub fn allocation_balance(&self) -> isize {
        self.allocation_balance
    }

    pub fn get_block_table<'a>(&'a self) -> &'a DownwardsVec<'static, Block> {
        &self.blocks
    }
    // Get the block table mutably. This is unsafe because it allows the block table to be modified.
    pub unsafe fn get_block_table_mut<'a>(&'a mut self) -> &'a mut DownwardsVec<'static, Block> {
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

#[inline]
fn align<T>(val: *mut T, downwards: bool) -> *mut T {
    align_with_alignment(val as *mut u8, mem::align_of::<T>(), downwards) as *mut T
}

fn align_with_alignment(val: *mut u8, alignment: usize, downwards: bool) -> *mut u8 {
    let val = val as usize;
    let offset = val % alignment;
    if offset == 0 {
        return val as *mut u8;
    }

    let ptr = if downwards {
        (val - offset) as *mut u8
    } else {
        (val + (alignment - offset)) as *mut u8
    };
    assert!(ptr.is_aligned());
    ptr
}

mod tests {

    use alloc::{boxed::Box, vec::Vec};

    use crate::memory::allocator::TEST_ALLOCATOR;

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
        assert!(!block.is_reusable);
        assert!(block.size() >= layout.size());
        assert_eq!(block.address, ptr.cast());
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

        assert_eq!(block.size, layout.size());
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

        assert_eq!(block.size, layout.size() * 4);
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

    #[kproc::test("Block Split")]
    fn test_block_split() {
        let layout = Layout::from_size_align(512, 1).unwrap();

        let allocator = &mut TEST_ALLOCATOR.get().blocks;

        let ptr = unsafe { allocator.allocate(layout) };

        alloc_check(ptr, layout, allocator);

        unsafe {
            allocator
                .deallocate(ptr, layout)
                .expect("Block failed to free");
        }

        let new_layout = Layout::from_size_align(128, 1).unwrap();

        let new_ptr = unsafe { allocator.allocate(new_layout) };

        alloc_check(new_ptr, new_layout, allocator);

        let new_block = allocator
            .find_block_by_ptr(unsafe { new_ptr.add(128) })
            .ok_or_else(|| {
                allocator.dbg_serial_send_csv();
                panic!("Block not found!")
            })
            .unwrap();

        assert_eq!(new_block.size, layout.size() - new_layout.size());
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
        for i in 0..200 {
            let mut vec = Vec::new_in(&TEST_ALLOCATOR);
            let layout = Layout::array::<u8>(100).expect("Failed to create layout");

            for i in 0..100 {
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
            assert!(!alloc.ptr_is_allocated(ptr));
            assert_eq!(alloc.allocation_balance, 0);
        }
    }
}
