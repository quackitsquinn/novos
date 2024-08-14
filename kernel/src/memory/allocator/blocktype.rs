use super::blocksize::BlockSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BlockType {
    Free(BlockSize),
    Allocated(BlockSize),
}

impl BlockType {
    pub fn size(&self) -> usize {
        match self {
            BlockType::Free(size) => size.as_usize(),
            BlockType::Allocated(size) => size.as_usize(),
        }
    }

    pub fn is_free(&self) -> bool {
        match self {
            BlockType::Free(_) => true,
            BlockType::Allocated(_) => false,
        }
    }

    pub fn block_size(&self) -> &BlockSize {
        match self {
            BlockType::Free(size) => size,
            BlockType::Allocated(size) => size,
        }
    }

    pub fn deallocate(self) -> Self {
        BlockType::Free(*self.block_size())
    }

    pub fn allocate(self) -> Self {
        BlockType::Allocated(*self.block_size())
    }
}
