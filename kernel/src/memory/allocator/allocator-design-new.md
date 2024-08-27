# Block based allocation (but its weird)

## The Block

Contains information about a segment of the heap. Rather than storing the block at the start of a segment of memory, we create an array at the end of the heap to store the blocks. This array (and encompassing struct) is currently named `Blocks` (but this is subject to change). Each block contains the following information:

- `size`: The size of the block
- `is_free`: Whether the block is free or not
- `address`: The address of the block
- `needs_delete`: Whether the block needs to be deleted or not

The `Blocks` struct contains the following information:

- `blocks`: The array of blocks
- `heap_start`: The start of the heap
- `heap_end`: The end of the heap
- `allocation_balance`: The balance of allocations and deallocations. This is intended as a debugging tool to see if there are any memory leaks. The larger the number, the more allocations there have been. The smaller the number, the more deallocations there have been. If the number is 0, then there have been an equal number of allocations and deallocations.
- `unmapped_start`: The start of the unmapped memory. This will slowly approach the end of the heap as more memory is allocated. If this reaches the end of the heap, and there are no free blocks, then the kernel has run out of memory.


If `blocks` becomes bigger than N, we run what is essentially a garbage collector to remove all the free blocks and compact the heap. (but not really)

### Why not use a `Option<Block>`?

The reason options aren't used is because of space. I was also running into ownership issues when I was trying to use `Option<Block>`. I could have used `Option<&mut Block>`, but that would have been a pain to work with.

### Semi-Garbage Collector

If `blocks` runs out of space, the last thing it will try is to allocate more. The first thing it does is to run the garbage collector.

1. Iterate through all the blocks and take note of all the free blocks
2. Join free blocks together that are sequential
3. Remove all the `None`s in the `blocks` array
4. Compact the blocks (remove all the free blocks)
5. If more space still needs to be allocated, allocate more space

Free blocks are important, because they are how we reclaim memory. 

## Allocation

When we allocate memory, we first check if there is a free block that is big enough. If there is, we use that block. If there isn't, we allocate a new block at the end of the heap.

1. Iterate through all the blocks and find a free block that is big enough
   1. If there are more than one free blocks that are next to each other, join them together. This will leave `None`s in the `blocks` array. (Moving the whole array is expensive)
   2. However, if the first free block is big enough, we can just use that block.
2. If the block found is over double the size of the requested size, split the block into two. The first block will be the requested size plus it's offset, and the second block will be the remaining size.
3. Set the block to be not free
4. Add the offset to the address of the block
5. Return the address of the block
   - This is undecided, but if there needs to be a certain alignment, we can return the address of the block + the offset

The block slice will **grow down** the heap. This is because the heap grows up, and the block slice is at the end of the heap.
  
## Deallocation

When memory is deallocated, the given pointer is checked to make sure it's both in the heap and not already free. If it is, the block is set to be free. If the next block is also free, the two blocks are joined together. This will be done to help reduce fragmentation (and to make the heap GC run less, because it's expensive).

## Debugging

Debugging is going to be integrated through the `Blocks` struct. There's no good in a broken allocator. The `Blocks` struct will contain the following utilities for debugging:

- `allocation_balance`: The balance of allocations and deallocations. This is intended as a debugging tool to see if there are any memory leaks. The larger the number, the more allocations there have been. The smaller the number, the more deallocations there have been. If the number is 0, then there have been an equal number of allocations and deallocations. This is a good way to test for memory leaks if you do something like this:

```rust
loop {
   assert!(allocation_balance == 0);
   black_box(vec![0u8; 30])
}
```

- `debug_serial_send_csv()`: This will send a CSV representation of the block table. Useful for visualizing the block table in ways that would be difficult to do in kernel space.
