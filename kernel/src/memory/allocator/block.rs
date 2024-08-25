use crate::memory::allocator::blocksize::BlockSize;

use super::blocktype::BlockType;
//INFO: We don't derive Copy to prevent accidental copies of the block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    // The type of the block
    pub block_type: BlockType,
    // The start address of the block
    pub address: usize,
    // If the block needs to be removed in the next block clean
    pub needs_delete: bool,
}

impl Block {
    pub fn new(block_type: BlockType, address: usize) -> Self {
        Self {
            block_type,
            address,
            needs_delete: false,
        }
    }

    pub fn size(&self) -> usize {
        self.block_type.size()
    }

    pub fn is_free(&self) -> bool {
        self.block_type.is_free()
    }

    pub fn deallocate(&mut self) {
        self.block_type = self.block_type.deallocate();
    }

    pub fn allocate(&mut self) {
        self.block_type = self.block_type.allocate();
    }

    pub fn split(&mut self, size: usize) -> Option<Block> {
        if self.size() < size {
            return None;
        }

        let new_block = Block::new(self.block_type, self.address + size);
        self.block_type = self.block_type.deallocate();
        Some(new_block)
    }

    pub fn merge(&mut self, other: &mut Block) -> Block {
        if other.address > self.address {
            return other.merge(self); // Ensure self is the block with the lower address
        }
        assert!(
            self.block_type.is_free() && other.block_type.is_free(),
            "Cannot merge allocated blocks"
        );
        Block::new(
            BlockType::Free(BlockSize::new_bytes(self.size() + other.size())),
            self.address,
        )
    }
}
