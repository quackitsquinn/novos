use core::{
    alloc::Layout,
    cell::UnsafeCell,
    mem,
    ptr::{self, NonNull},
    slice,
};

use log::{debug, error, info, trace};

use crate::{debug_release_check, sprintln};

use super::{
    block::{self, Block},
    downwards_vec::DownwardsVec,
};

#[derive(Debug)]
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

    unsafe fn init_table_at(
        ptr: *mut Block,
        table_size: usize,
    ) -> (&'static mut Block, DownwardsVec<'static, Block>) {
        let block_table_base = unsafe { ptr.sub(table_size) };
        let size_bytes = table_size * mem::size_of::<Block>();
        info!(
            "Creating block table with size {} at {:p}",
            BLOCK_SIZE_BYTES, ptr
        );
        // Set the first block to contain itself
        let block = Block::new(size_bytes, block_table_base as *mut u8, false);
        let mut blocks = unsafe { DownwardsVec::new(ptr, INIT_BLOCK_SIZE) };

        info!("Pushing table block");
        blocks.push(block).expect("Failed to push block");
        info!("Pushed table block");

        let table_block = unsafe { &mut *blocks.as_mut_ptr() };
        (table_block, blocks)
    }

    unsafe fn push_block(&mut self, block: Block) {
        self.check_block_space();

        // TODO: Use the actual allocator design here. This is a temporary solution because I got tired of fighting with pointers.
        for blks in &mut *self.blocks {
            if blks.is_reusable {
                *blks = block;

                return;
            }
        }

        self.blocks.push(block).expect("Failed to push block");
    }

    // This will be relatively slow, but it should be called less and less as the heap grows.
    unsafe fn allocate_block(&mut self, size: usize) -> Block {
        self.dbg_print_blocks();
        let block = Block::new(size, self.unmap_start as *mut u8, false);
        self.unmap_start += size;
        self.dbg_print_blocks();
        block
    }

    unsafe fn find_block_by_ptr(&mut self, ptr: *mut u8) -> Option<&mut Block> {
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

    pub unsafe fn allocate(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let alignment = layout.align();
        let mut split = None;
        let mut address = ptr::null_mut();

        for block in &mut *self.blocks {
            if block.is_free() && block.size() + alignment >= size {
                block.allocate();
                address = block.address as *mut u8;
                if block.size() > size + (size as f64 * SPLIT_THRESHOLD) as usize {
                    split = block.split(size + alignment);
                }
            }
        }

        if address.is_null() {
            sprintln!("Allocating new block");
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
        trace!("Allocated block at {:#x}", ptr as usize);
        ptr
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        if let Some(blk) = // OLD: align_with_alignment(ptr, layout.align(), true))
            unsafe { self.find_block_by_ptr(ptr) }
        {
            info!("Deallocating block {:?}", blk);
            if blk.is_free() {
                // TODO: Should DEFINITELY not panic here. Tis is a temporary debug check.
                debug_release_check! {
                    debug {
                        error!("Block already deallocated");
                        self.dbg_serial_send_csv();
                        //panic!("Block already deallocated");
                    },
                    release {
                        error!("Block already deallocated");
                    }
                }
                return;
            }
            blk.deallocate();
            info!("Deallocated block {:?}", blk);
            self.allocation_balance -= 1;
            self.dbg_print_blocks();
            return;
        }
        debug_release_check!(
            debug {
                panic!("Block not found");
            },
            release {
                error!("Block not found");
            }
        )
    }

    unsafe fn gc(&mut self) {
        let mut last_free_block: Option<&mut Block> = None;
        let mut joined = 0;
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

    unsafe fn ptr_is_allocated(&self, ptr: *mut u8) -> bool {
        let ptr = ptr as usize;
        for block in &*self.blocks {
            let addr = block.address as usize;
            if ptr >= addr && ptr < addr + block.size() {
                return block.is_free();
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
            panic!("Out of block space"); // TODO: This should try to reserve more space.
        }
    }

    fn run_gc(&mut self) {
        sprintln!("START-PRE-GC");
        self.dbg_serial_send_csv();
        sprintln!("END-PRE-GC");
        unsafe {
            self.gc();
        }
        sprintln!("START-POST-GC");
        self.dbg_serial_send_csv();
        sprintln!("END-POST-GC");
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

//#[cfg(test)]
mod tests {
    use crate::stack_check;

    use super::*;
    use kproc::test;
    use mem::{ManuallyDrop, MaybeUninit};

    /// A test allocator that uses a fixed-size array for blocks.
    /// ***WARNING: This is not a real allocator. DO NOT DEREF ANY POINTERS FROM THIS ALLOCATOR.***
    pub struct DebugAllocator<const SPACE: usize> {
        pub inner: BlockAllocator,
        inner_arr: [ManuallyDrop<Block>; SPACE],
    }

    impl<const SPACE: usize> DebugAllocator<SPACE> {
        pub fn new(heap_start: usize, heap_end: usize) -> Self {
            // This is safe because A. BlockAllocator expects uninitialized memory and B. ManuallyDrop is repr(transparent).
            trace!(
                "Creating DebugAllocator with heap start: {:#x} and heap end: {:#x}",
                heap_start,
                heap_end,
            );
            let mut inner_arr: [ManuallyDrop<Block>; SPACE] =
                unsafe { MaybeUninit::uninit().assume_init() };
            // This is OK because we don't reference the inner allocator until it's initialized.
            #[allow(invalid_value)]
            let mut this: DebugAllocator<SPACE> = unsafe { MaybeUninit::uninit().assume_init() };
            this.inner_arr = inner_arr;
            stack_check();
            this.inner = unsafe {
                BlockAllocator::init_at(
                    heap_start,
                    heap_end,
                    this.inner_arr.as_mut_ptr() as *mut Block,
                    SPACE,
                )
            };
            trace!("Created DebugAllocator");
            this
        }
    }

    #[test("DebugAllocator is valid", can_recover = true)]
    fn test_debug_allocator() {
        stack_check();
        let mut allocator = DebugAllocator::<10>::new(0x1000, 0x2000);
        assert_eq!(allocator.inner.get_block_table().len(), 1);
        assert_eq!(
            allocator.inner.get_block_table()[0].size(),
            BLOCK_SIZE_BYTES
        );
    }
}
