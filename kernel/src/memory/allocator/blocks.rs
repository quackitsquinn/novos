use core::{
    alloc::{GlobalAlloc, Layout},
    error,
    mem::{self, MaybeUninit},
    ptr::{self, addr_eq, addr_of},
    slice,
};

use log::{debug, error, info, trace};

use crate::{assert_or_else, debug_release_check, sprintln};

use super::{block::Block, blocksize::BlockSize, blocktype::BlockType};

pub struct Blocks {
    // INFO: We don't use a Vec here because A. infinite recursion and B. The slice grows downwards rather than upwards.
    blocks: &'static mut [Block], // TODO: maybe use MaybeUninits? Would half the size of the struct.
    unmap_start: usize,           // The start of the unmapped memory.
    heap_start: usize,
    heap_end: usize,
    map_unmap_balance: isize,
}
// TODO: Don't implement Send and Sync for Blocks. Implement it for LockedAllocator instead.
unsafe impl Send for Blocks {}
unsafe impl Sync for Blocks {}

const INIT_BLOCK_SIZE: usize = 1024 * 10; // 10KB
const SPLIT_THRESHOLD: f64 = 0.5;

impl Blocks {
    pub unsafe fn init(heap_start: usize, heap_end: usize) -> Self {
        let block_heap_end = heap_end - mem::size_of::<Block>();
        let block_heap_end = align(block_heap_end as *mut Block, true) as usize;
        sprintln!("Block heap end: {:#x}", block_heap_end);
        // Set the first block to contain itself
        let block = Block::new(
            BlockType::Allocated(BlockSize::new_bytes(INIT_BLOCK_SIZE)),
            block_heap_end as *mut u8,
        );
        sprintln!("Writing initblock to {:#x}", block_heap_end);
        unsafe {
            let mut lastblock = (block_heap_end) as *mut Block;
            lastblock.write(block);
        };
        sprintln!("Wrote initblock to {:#x}", block_heap_end);
        let arrptr = block_heap_end as *mut Block;
        sprintln!("Block array at {:#x}", arrptr as usize);
        let blocks = unsafe { slice::from_raw_parts_mut(arrptr, 1) };
        sprintln!("Blocks: {:?}", blocks);

        Self {
            blocks,
            heap_start,
            heap_end,
            unmap_start: heap_start,
            map_unmap_balance: 0,
        }
    }

    pub unsafe fn push_block(&mut self, block: Block) {
        self.run_gc_if_needed();
        let off =
            (self.heap_end - (mem::size_of::<Block>() * (self.blocks.len() + 1))) as *mut Block;
        let off = align(off, true);
        unsafe {
            off.write(block);
            self.blocks = slice::from_raw_parts_mut(off, self.blocks.len() + 1);
        }
    }
    // This will be relatively slow, but it should be called less and less as the heap grows.
    unsafe fn allocate_block(&mut self, size: usize) -> Block {
        let block = Block::new(
            BlockType::Allocated(BlockSize::new_bytes(size)),
            self.unmap_start as *mut u8,
        );
        self.unmap_start += size;
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
        let align = layout.align();
        let mut split = None;
        let mut address = ptr::null_mut();

        for block in &mut *self.blocks {
            if block.is_free() && block.size() + align >= size {
                block.allocate();
                address = block.address as *mut u8;
                if block.size() > size + align + (size as f64 * SPLIT_THRESHOLD) as usize {
                    split = block.split(size + align);
                }
            }
        }

        if address.is_null() {
            sprintln!("Allocating new block");
            // Allocate a new block
            let block = unsafe { self.allocate_block(size + align) };
            address = block.address as *mut u8;
            unsafe { self.push_block(block) };
        }

        if let Some(block) = split {
            unsafe { self.push_block(block) };
        }

        let addr = address as usize;

        self.map_unmap_balance += 1;

        let ptr = (addr + address.align_offset(align)) as *mut u8;
        trace!("Allocated block at {:#x}", ptr as usize);
        self.run_gc();
        ptr
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        if let Some(blk) = unsafe { self.find_block_by_ptr(ptr) } {
            info!("Deallocating block {:?}", blk);
            if blk.is_free() {
                // TODO: Should DEFINITELY not panic here. Tis is a temporary debug check.
                debug_release_check! {
                    debug {
                        panic!("Block already deallocated");
                    },
                    release {
                        error!("Block already deallocated");
                    }
                }
            }
            blk.deallocate();
            info!("Deallocated block {:?}", blk);
            self.map_unmap_balance -= 1;
            self.dbg_print_blocks();
            return;
        }
        sprintln!(
            "Block not found for deallocation \n BLOCKS: {:?}",
            self.blocks
        );

        // sprintln!(
        //     "Is ptr in heap? {}",
        //     ptr >= self.heap_start && ptr < self.heap_end
        // );

        error!("Block not found for deallocation");
    }

    unsafe fn run_join(&mut self) {
        let mut last_free_block: Option<&mut Block> = None;
        let mut joined = 0;
        for block in &mut *self.blocks {
            if !block.needs_delete {
                if block.is_free() {
                    if let Some(lsblk) = last_free_block {
                        info!("Joining blocks {:?} and {:?}", lsblk, block);
                        *block = block.merge(lsblk);
                        joined += 1;
                        lsblk.needs_delete = true
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

    unsafe fn run_shrink(&mut self) {
        // TODO: better optimization for sequential deletion blocks
        let mut len = self.blocks.len();
        let mut base = self.blocks.as_mut_ptr();
        let mut deleted = 0;
        self.dbg_print_blocks();
        // step -> if block is marked as needs_delete -> move all elements before 1 up -> continue
        for (i, block) in self.blocks.iter_mut().enumerate() {
            if block.needs_delete {
                debug!("Deleting block {:?}", block);
                if i == 0 {
                    base = unsafe { base.add(1) };
                    len -= 1;
                    deleted += 1;
                    continue;
                }
                // Copy base + i - 1 to i
                unsafe {
                    assert_or_else!(
                        base.add(1) as usize <= self.heap_end
                            && base.add(1) as usize + size_of::<Block>() * len <= self.heap_end,
                        {
                            error!("Unable to shrink block table! (Attempting to copy 0x{:X} blocks to {:p} would overrun the heap by 0x{:X})", len, base.add(1),(base.add(1) as usize + size_of::<Block>() * len) - self.heap_end);
                            self.dbg_print_blocks();
                            self.dbg_serial_send_csv();
                            panic!("Unable to shrink block table!");
                        }
                    );
                    ptr::copy(base, base.add(1), i);
                    base = base.add(1);
                }
                len -= 1;
                deleted += 1;
            }
        }
        self.dbg_print_blocks();
        debug!("Deleted {} blocks", deleted);
        self.blocks =
            unsafe { slice::from_raw_parts_mut(self.blocks.as_mut_ptr().add(deleted), len) }
    }

    fn run_gc_if_needed(&mut self) {
        if self.blocks.len() * mem::size_of::<Block>() >= self.blocks.last().unwrap().size() {
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
            self.run_shrink();
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
            self.map_unmap_balance
        );

        let unalloc_count = self.blocks.iter().filter(|b| b.is_free()).count();
        trace!(
            "unallocs: {} allocs: {}",
            unalloc_count,
            self.blocks.len() - unalloc_count
        );
    }

    fn dbg_serial_send_csv(&self) {
        sprintln!("address,size,free,delete");
        for block in &*self.blocks {
            sprintln!(
                "{:p},{},{},{}",
                block.address,
                block.size(),
                block.is_free(),
                block.needs_delete
            );
        }
    }
}

fn align<T>(val: *mut T, downwards: bool) -> *mut T {
    let align = mem::align_of::<T>();
    let val = val as usize;
    let offset = val % align;
    if offset == 0 {
        return val as *mut T;
    }
    if downwards {
        (val - offset) as *mut T
    } else {
        (val + (align - offset)) as *mut T
    }
}
