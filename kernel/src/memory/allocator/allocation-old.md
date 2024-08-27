# Block based allocation (but its weird)

## The Block

Contains information about a segment of the heap. Rather than storing the block at the start of a block, we create an array at the end of the heap to store the blocks. This array is called `blocks`. Each block contains the following information:

- `size`: The size of the block
- `is_free`: Whether the block is free or not
- `address`: The address of the block

If `blocks` becomes bigger than N, we run what is essentially a garbage collector to remove all the free blocks and compact the heap. (but not really)

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

TODO: Alignment

1. Iterate through all the blocks and find a free block that is big enough
   1. If there are more than one free blocks that are next to each other, join them together. This will leave `None`s in the `blocks` array. (Moving the whole array is expensive)
   2. However, if the first free block is big enough, we can just use that block.
2. If the block found is over double the size of the requested size, split the block into two. The first block will be the requested size, and the second block will be the remaining size.
3. Set the block to be not free
4. Return the address of the block
   - This is undecided, but if there needs to be a certain alignment, we can return the address of the block + the offset
  
## Deallocation

When we deallocate memory, we first check if the block is in the heap. If it is, we set the block to be free. (if we are handed a garbage address, we quietly ignore it) We will also join the block with the next block if it is free.
