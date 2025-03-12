# Block based allocator

## Overview

This allocator is based on allocating custom sized blocks of memory. It is a mix between standard fixed block allocators and linked list based allocators. This allocator is solely designed by me and is probably not the best one out there, but it works for my use case. (ignoring the months of debugging)

## How it works

### Block

A block is a custom sized chunk of memory. It is defined as follows:

```rust
pub struct Block {
    /// The size of the block
    pub size: usize,
    ///  Is the block free or allocated
    pub is_free: bool,
    /// The start address of the block
    pub address: *mut u8,
}
```

### Block Table

The block table is a fixed capacity vector of blocks defined at the start of the heap. (TODO: Maybe have it be a custom pointer? There's probably a better way to do this)

Keeping the block table at the start of the heap prevents memory corruption if the allocator breaks.

### Initialization

The block table is initialized with a constant capacity of 512 blocks. Then, the *first* block is created to contain the entire block table, and marked as used. Then, the *second* block is created to contain the rest of the heap, and marked as free.

### Allocation

The allocator will iterate through the block table and find the first free block that is large enough to contain the requested size. If it finds one, it will split the block into two blocks: one for the requested size and one for the remaining size. The remaining block will be marked as free and added back to the block table.

### Deallocation

The allocator will iterate through the block table and find the block that contains the address to be deallocated. If it finds one, it will mark the block as free.

### Defragmentation / Garbage Collection

If the allocator is unable to find a free block that is large enough to contain the requested size or has run out of block table space, it will defragment the heap. This is done by iterating through the block table and merging adjacent free blocks into one larger block. This process is repeated until the size of the block table stops decreasing.
If the block table is still full after defragmentation, the allocator will panic.