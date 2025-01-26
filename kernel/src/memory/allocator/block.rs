//INFO: We don't derive Copy to prevent accidental copies of the block
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)] // Prevent field reordering
pub struct Block {
    // The size of the block
    pub size: usize,
    //  Is the block free or allocated
    pub is_free: bool,
    // The start address of the block
    pub address: *mut u8,
}

impl Block {
    pub fn new(size: usize, address: *mut u8, is_free: bool) -> Self {
        Self {
            size,
            is_free: is_free,
            address,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_free(&self) -> bool {
        self.is_free
    }

    pub fn deallocate(&mut self) {
        self.is_free = true;
    }

    pub fn allocate(&mut self) {
        self.is_free = false;
    }
    /// Split the block into two blocks. This block will have the size of `size` and the new block will have the remaining size.
    pub fn split(&mut self, size: usize) -> Option<Block> {
        if self.size() < size {
            return None;
        }

        let new_block = Block::new(self.size - size, unsafe { self.address.add(size) }, true);
        self.size = size;
        Some(new_block)
    }

    pub fn merge(&mut self, other: &mut Block) -> Block {
        // Ensure the blocks are free and reusable
        debug_assert_eq!(self.is_free, other.is_free);
        //debug_assert!(self.is_reusable && other.is_reusable);

        if other.address < self.address {
            return other.merge(self); // Ensure self is the block with the lower address
        }

        debug_assert!(self.is_adjacent(other));

        let new_size = self.size + other.size;

        Block::new(new_size, self.address, true)
    }

    pub fn is_adjacent(&self, other: &Block) -> bool {
        let self_end = self.address as usize + self.size;

        self_end == other.address as usize
            || self.address as usize == other.address as usize + other.size
    }
}

impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.address.partial_cmp(&other.address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[kproc::test("Block split", can_recover = true)]
    fn test_block_split() {
        let mut block = Block::new(1024, 0x1000 as *mut u8, true);
        let new_block = block.split(512).unwrap();

        assert_eq!(block.size(), 512);
        assert_eq!(new_block.size(), 512);
        assert_eq!(new_block.address, 0x1200 as *mut u8);
        assert_eq!(
            block.address as usize + block.size,
            new_block.address as usize
        );
    }

    #[kproc::test("Block split too large", can_recover = true)]
    fn test_block_split_too_large() {
        let mut block = Block::new(1024, 0x1000 as *mut u8, true);
        let new_block = block.split(2048);

        assert!(!new_block.is_none());
        assert_eq!(block.size(), 1024);
    }

    #[kproc::test("Block split not even", can_recover = true)]
    fn test_block_split_not_even() {
        let mut block = Block::new(1024, 0x1000 as *mut u8, true);
        let new_block = block.split(513).unwrap();

        assert_eq!(block.size(), 513);
        assert_eq!(new_block.size(), 511);
        assert_eq!(new_block.address, 0x1201 as *mut u8);
        assert_eq!(
            block.address as usize + block.size,
            new_block.address as usize
        );
    }

    #[kproc::test("Block merge", can_recover = true)]
    fn test_block_merge() {
        let mut block1 = Block::new(512, 0x1000 as *mut u8, true);
        let mut block2 = Block::new(512, 0x1200 as *mut u8, true);

        let new_block = block1.merge(&mut block2);

        assert_eq!(new_block.size(), 1024);
        assert_eq!(new_block.address, block1.address);
    }

    #[kproc::test("Block is adjacent", can_recover = true)]
    fn test_block_is_adjacent() {
        let block1 = Block::new(512, 0x1000 as *mut u8, true);
        let block2 = Block::new(512, 0x1200 as *mut u8, true);

        assert!(block1.is_adjacent(&block2));
    }

    #[kproc::test("Block is not adjacent", can_recover = true)]
    fn test_block_is_not_adjacent() {
        let block1 = Block::new(512, 0x1000 as *mut u8, true);
        let block2 = Block::new(512, 0x1600 as *mut u8, true);

        assert!(!block1.is_adjacent(&block2));
    }
}
