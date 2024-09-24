//INFO: We don't derive Copy to prevent accidental copies of the block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    // The size of the block
    pub size: usize,
    //  Is the block free or allocated
    pub is_free: bool,
    // The start address of the block
    pub address: *mut u8,
    // Can the block be reused
    pub is_reusable: bool,
}

impl Block {
    pub fn new(size: usize, address: *mut u8, is_free: bool) -> Self {
        Self {
            size,
            is_free: is_free,
            address,
            is_reusable: false,
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

        if other.address > self.address {
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

    pub fn set_reusable(&mut self, reusable: bool) {
        debug_assert!(self.is_free);
        self.is_reusable = reusable;
    }
}
