use core::{alloc::Layout, mem, ptr, slice};

use log::{debug, error, info, trace};

use crate::{debug_release_check, sprintln};

use super::block::Block;

#[derive(Debug)]
pub struct Blocks {
    // INFO: We don't use a Vec here because A. infinite recursion and B. The slice grows downwards rather than upwards.
    blocks: &'static mut [Block],
    unmap_start: usize, // The start of the unmapped memory.
    heap_start: usize,
    heap_end: usize,
    allocation_balance: isize,
}
// TODO: Don't implement Send and Sync for Blocks. Implement it for LockedAllocator instead.
unsafe impl Send for Blocks {}
unsafe impl Sync for Blocks {}

const INIT_BLOCK_SIZE: usize = 1024 * 10; // 10KB
const SPLIT_THRESHOLD: f64 = 0.5;
const GC_THRESHOLD: f64 = 0.8;

impl Blocks {
    pub unsafe fn init(heap_start: usize, heap_end: usize) -> Self {
        let block_heap_end = heap_end - mem::size_of::<Block>();
        let block_heap_end = align(block_heap_end as *mut Block, true) as usize;
        sprintln!("Block heap end: {:#x}", block_heap_end);
        // Set the first block to contain itself
        let block = Block::new(INIT_BLOCK_SIZE, block_heap_end as *mut u8, false);
        sprintln!("Writing block table block to {:#x}", block_heap_end);
        unsafe {
            let block_table_block = (block_heap_end) as *mut Block;
            block_table_block.write(block);
        };
        sprintln!("Wrote block table block to {:#x}", block_heap_end);
        let blk_tbl_ptr = block_heap_end as *mut Block;
        sprintln!("Block table at {:#x}", blk_tbl_ptr as usize);
        let blocks = unsafe { slice::from_raw_parts_mut(blk_tbl_ptr, 1) };
        sprintln!("Blocks: {:?}", blocks);

        Self {
            blocks,
            heap_start,
            heap_end,
            unmap_start: heap_start,
            allocation_balance: 0,
        }
    }

    pub unsafe fn push_block(&mut self, block: Block) {
        self.check_block_space();
        // TODO: Use the actual allocator design here. This is a temporary solution because I got tired of fighting with pointers.
        for blks in &mut *self.blocks {
            if blks.is_reusable {
                *blks = block;
                return;
            }
        }
        // If this is reached, we just need to push the block to the end of the slice.
        let off = unsafe { self.blocks.as_mut_ptr().sub(1) };
        // SAFETY: run_gc_if_needed ensures that there is enough space for the block.
        unsafe {
            off.write(block);
            self.blocks = slice::from_raw_parts_mut(off, self.blocks.len() + 1);
        }
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
            if ptr >= addr && ptr <= addr + block.size() {
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
                if block.size() > size + alignment + (size as f64 * SPLIT_THRESHOLD) as usize {
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
        if let Some(blk) =
            unsafe { self.find_block_by_ptr(align_with_alignment(ptr, layout.align(), true)) }
        {
            info!("Deallocating block {:?}", blk);
            if blk.is_free() {
                // TODO: Should DEFINITELY not panic here. Tis is a temporary debug check.
                debug_release_check! {
                    debug {
                        panic!("Block already deallocated");
                    },
                    release {
                        error!("Block already deallocated");
                        return;
                    }
                }
            }
            blk.deallocate();
            info!("Deallocated block {:?}", blk);
            self.allocation_balance -= 1;
            self.dbg_print_blocks();
            return;
        }
        sprintln!(
            "Block not found for deallocation \n BLOCKS: {:?}",
            self.blocks
        );
        error!(
            "Block not found for deallocation (ptr: {:#x})",
            ptr as usize
        );
    }

    unsafe fn run_join(&mut self) {
        let mut last_free_block: Option<&mut Block> = None;
        let mut joined = 0;
        for block in &mut *self.blocks {
            if !block.is_reusable {
                if block.is_free() {
                    if let Some(lsblk) = last_free_block {
                        info!("Joining blocks {:?} and {:?}", lsblk, block);
                        *block = block.merge(lsblk);
                        joined += 1;
                        lsblk.is_reusable = true
                    }
                    last_free_block = Some(block)
                } else {
                    last_free_block = None;
                }
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
        // run GC if the block table is more than GC_THRESHOLD full
        if (self.blocks.len() * size_of::<Block>()) as f64
            / self.blocks.last().unwrap().size() as f64
            >= GC_THRESHOLD
        {
            self.run_gc();
        }

        if self.blocks.len() * mem::size_of::<Block>() >= self.blocks.last().unwrap().size() {
            // TODO: handle this better. Should not panic.
            panic!("Out of memory");
        }
    }

    fn run_gc(&mut self) {
        sprintln!("START-PRE-GC");
        self.dbg_serial_send_csv();
        sprintln!("END-PRE-GC");
        unsafe {
            self.run_join();
        }
        sprintln!("START-POST-GC");
        self.dbg_serial_send_csv();
        sprintln!("END-POST-GC");
    }

    fn dbg_print_blocks(&self) {
        let mx = self.blocks.last().unwrap().size() / size_of::<Block>();
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
