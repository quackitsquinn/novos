use core::{
    alloc::{GlobalAlloc, Layout},
    mem, ptr, slice,
};

use log::{debug, trace};

use crate::sprintln;

use super::{block::Block, blocksize::BlockSize, blocktype::BlockType};

pub struct Blocks {
    // INFO: We don't use a Vec here because A. infinite recursion and B. The slice grows downwards rather than upwards.
    blocks: &'static mut [Block], // TODO: maybe use MaybeUninits? Would half the size of the struct.
    unmap_start: usize,           // The start of the unmapped memory.
    heap_start: usize,
    heap_end: usize,
}

const INIT_BLOCK_SIZE: usize = 1024 * 10;
const SPLIT_THRESHOLD: f64 = 0.5;

impl Blocks {
    pub unsafe fn init(heap_start: usize, heap_end: usize) -> Self {
        let block_heap_end = heap_end - mem::size_of::<Block>();
        let block_heap_end = align(block_heap_end as *mut Block, true) as usize;
        sprintln!("Block heap end: {:#x}", block_heap_end);
        // Set the first block to contain itself
        let block = Block::new(
            BlockType::Allocated(BlockSize::new_bytes(INIT_BLOCK_SIZE)),
            block_heap_end,
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
        }
    }

    pub unsafe fn push_block(&mut self, block: Block) {
        self.check_block_space();
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
            self.unmap_start,
        );
        self.unmap_start += size;
        block
    }

    unsafe fn find_block_by_ptr(&mut self, ptr: *mut u8) -> Option<&mut Block> {
        let ptr = ptr as usize;
        for block in &mut *self.blocks {
            if ptr >= block.address && ptr < block.address + block.size() {
                trace!(
                    "Found ptr ({:#x}) in block {:#x} (off: {})",
                    ptr,
                    block.address,
                    ptr - block.address
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
        (addr + address.align_offset(align)) as *mut u8
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        let align = layout.align();
        let ptr = ptr as usize - align;

        if let Some(blk) = unsafe { self.find_block_by_ptr(ptr as *mut u8) } {
            if blk.is_free() {
                return;
            }
            blk.deallocate();
            self.dbg_print_blocks();
            return;
        }
    }

    unsafe fn run_join(&mut self) {
        let mut last_free_block: Option<&mut Block> = None;
        let mut joined = 0;
        for block in &mut *self.blocks {
            if !block.needs_delete {
                if block.is_free() {
                    if let Some(lsblk) = last_free_block {
                        *block = block.merge(lsblk);
                        joined += 1;
                        lsblk.needs_delete = true
                    }
                    last_free_block = Some(block)
                }
            }
        }
        debug!("Joined {} blocks", joined);
    }

    unsafe fn ptr_is_allocated(&self, ptr: *mut u8) -> bool {
        let ptr = ptr as usize;
        for block in &*self.blocks {
            if ptr >= block.address && ptr < block.address + block.size() {
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
                // Copy base + i - 1 to i
                unsafe {
                    ptr::copy(base, base.add(1), i - 1);
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

    fn check_block_space(&mut self) {
        if self.blocks.len() * mem::size_of::<Block>() >= self.blocks.last().unwrap().size() {
            unsafe {
                self.run_join();
                self.run_shrink();
            };
        }

        if self.blocks.len() * mem::size_of::<Block>() >= self.blocks.last().unwrap().size() {
            // TODO: handle this better. Should not panic.
            panic!("Out of memory");
        }
    }

    fn dbg_print_blocks(&self) {
        let mx = self.blocks.last().unwrap().size() / size_of::<Block>();
        trace!(
            "blkcount {}/{} ({})",
            self.blocks.len(),
            mx,
            self.blocks.len() as f64 / mx as f64
        );

        let unalloc_count = self.blocks.iter().filter(|b| b.is_free()).count();
        trace!(
            "unallocs: {} allocs: {}",
            unalloc_count,
            self.blocks.len() - unalloc_count
        );
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
