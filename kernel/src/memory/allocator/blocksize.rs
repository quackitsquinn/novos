use super::{block::Block, blocktype::BlockType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockSize(usize);

impl BlockSize {
    // Creates a new block size in kilobytes.
    pub fn new_kb(kb: usize) -> Self {
        BlockSize(kb * 1024)
    }

    // Creates a new block size in bytes.
    pub fn new_bytes(bytes: usize) -> Self {
        BlockSize(bytes)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn will_fit(&self, size: usize) -> bool {
        size <= self.0
    }

    fn block_will_fit(&self, block: &Block) -> bool {
        match &block.block_type {
            BlockType::Free(block_size) => block_size.will_fit(self.0),
            BlockType::Allocated(block_size) => block_size.will_fit(self.0),
        }
    }

    fn size_will_fit(&self, other: &BlockSize) -> bool {
        self.0 <= other.0
    }
}
