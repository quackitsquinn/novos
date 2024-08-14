use core::{mem, slice};

use super::{block::Block, blocksize::BlockSize, blocktype::BlockType};

pub struct Blocks {
    // INFO: We don't use a Vec here because A. infinite recursion and B. The slice grows downwards rather than upwards.
    blocks: &'static mut [Option<Block>], // TODO: maybe use MaybeUninits? Would half the size of the struct.
    heap_start: usize,
    heap_end: usize,
}

const INIT_BLOCK_SIZE: usize = 1024 * 10;

impl Blocks {
    pub unsafe fn init(heap_start: usize, heap_end: usize) -> Self {
        let block_heap_end = heap_end - mem::size_of::<Block>();
        // Set the first block to contain itself
        let block = Some(Block::new(
            BlockType::Allocated(BlockSize::new_bytes(INIT_BLOCK_SIZE)),
            block_heap_end,
        ));
        unsafe {
            let mut lastblock = (block_heap_end) as *mut Block;
            lastblock.write(block.unwrap());
        };
        let blocks = unsafe { slice::from_raw_parts_mut(block_heap_end as *mut Block, 1) };

        Self {
            blocks: unsafe { mem::transmute(blocks) }, // I thiiink this is safe?
            heap_start,
            heap_end,
        }
    }
}
