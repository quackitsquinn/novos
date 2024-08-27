//INFO: We don't derive Copy to prevent accidental copies of the block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    // The size of the block
    pub size: usize,
    //  Is the block free or allocated
    pub is_free: bool,
    // The start address of the block
    pub address: *mut u8,
    // If the block needs to be removed in the next block clean
    pub needs_delete: bool,
}

impl Block {
    pub fn new(size: usize, address: *mut u8, is_free: bool) -> Self {
        Self {
            size,
            is_free: is_free,
            address,
            needs_delete: false,
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
        if other.address > self.address {
            return other.merge(self); // Ensure self is the block with the lower address
        }
        assert!(
            self.is_free() && other.is_free(),
            "Cannot merge allocated blocks"
        );
        Block::new(self.size() + other.size(), self.address, true)
    }
}
