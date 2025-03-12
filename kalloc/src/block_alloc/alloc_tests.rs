use core::{mem, pin::Pin, ptr::NonNull};
use std::alloc::{Global, dealloc};

use alloc::{boxed::Box, vec::Vec};

use crate::{GlobalAllocatorWrapper, test_common::DeferDealloc};

use super::*;
// BlockAllocator requires a heap size of 0x3200. Add 0x100 for a little extra space.
// Must be divisible by 8
const ARENA_SIZE: usize = 0x10000;

fn get_allocator<const SIZE: usize>() -> (BlockAllocator, DeferDealloc) {
    // This is to theoretically fix some issues with the allocator really not liking lower alignments ???
    let (alloc_allocation, ptr) =
        DeferDealloc::alloc(Layout::from_size_align(SIZE, 2 << 11).unwrap());
    let start = ptr.as_ptr().cast::<u8>();
    let end = unsafe { start.add(SIZE) };
    let alloc = unsafe { BlockAllocator::init(start.cast(), end.cast()) };
    (alloc, alloc_allocation)
}
/// Gets a block allocator wrapped in a global allocator wrapper, so it implements GlobalAlloc + Allocator.
fn get_full_allocator<const SIZE: usize>() -> (GlobalAllocatorWrapper<BlockAllocator>, DeferDealloc)
{
    let (alloc, vec) = get_allocator::<SIZE>();
    let gaw = GlobalAllocatorWrapper::new();
    gaw.init(|| alloc);
    (gaw, vec)
}

#[track_caller]
fn alloc_check<T>(ptr: *mut T, layout: Layout, allocator: &BlockAllocator) {
    min_check(ptr, layout, allocator);
    let block = allocator
        .find_block_by_ptr(ptr.cast())
        .expect("Allocated pointer not found");
    // Check if whole range is allocated
    for i in 0..layout.size() {
        assert!(
            allocator.ptr_is_allocated(unsafe { ptr.cast::<u8>().add(i).cast() }),
            "Failed at {}! (Block: {:#?})",
            i,
            block
        );
    }
}
#[track_caller]
fn min_check<T>(ptr: *mut T, layout: Layout, allocator: &BlockAllocator) {
    assert!(!ptr.is_null(), "Pointer is null");
    let block = allocator
        .find_block_by_ptr(ptr.cast())
        .expect("Allocated pointer not found");
    assert!(!block.is_free, "Block marked as free");
    if layout.align() != 1 {
        assert!(
            block.size >= layout.size() + layout.align(),
            "Block size too small"
        );
    } else {
        assert!(block.size >= layout.size(), "Block size too small");
    }
    assert!(ptr.is_aligned(), "Pointer is unaligned");
    // Make sure the block table is not overwritten
    let block_table_ptr = &allocator.table_block;
    assert!(
        block_table_ptr.address > ptr.cast(),
        "Block table overwritten"
    );
    assert!(
        block_table_ptr.address > unsafe { ptr.cast::<u8>().add(block.size) },
        "Block table overwritten"
    );
    allocator.condition_check();
}

#[cfg(test)]
#[ctor::ctor]
static INIT: () = {
    let _ = env_logger::builder().is_test(true).try_init();
    crate::enable_logging();
};

#[test]
fn test_allocation() {
    let layout = Layout::from_size_align(512, 1).unwrap();

    let (mut allocator, _defer_guard) = get_allocator::<ARENA_SIZE>();

    let ptr = unsafe { allocator.allocate(layout) };

    alloc_check(ptr, layout, &mut allocator);

    unsafe { allocator.deallocate(ptr, layout) }.expect("Block failed to free");
    assert_eq!(allocator.allocation_balance, 0);
    assert!(!allocator.ptr_is_allocated(ptr));

    let block = allocator
        .find_block_by_ptr(ptr)
        .expect("Block pointer not found");

    assert!(block.size >= layout.size());
    assert!(block.is_free);
}

#[test]
fn test_block_join() {
    let layout = Layout::from_size_align(512, 1).unwrap();

    let (mut allocator, _defer_guard) = get_allocator::<ARENA_SIZE>();

    let ptrs = [
        unsafe { allocator.allocate(layout) },
        unsafe { allocator.allocate(layout) },
        unsafe { allocator.allocate(layout) },
        unsafe { allocator.allocate(layout) },
    ];

    for ptr in &ptrs {
        alloc_check(*ptr, layout, &allocator);
    }

    for ptr in &ptrs {
        unsafe { allocator.deallocate(*ptr, layout) }.expect("Block failed to free");
    }

    // Manually run GC
    allocator.defrag();

    let block = allocator
        .find_block_by_ptr(ptrs[0])
        .expect("Block pointer not found");
    assert!(block.size == layout.size() * 4);
    allocator.print_state();
    assert!(block.is_free);
}

#[test]
fn test_block_reuse() {
    let layout = Layout::from_size_align(512, 1).unwrap();

    let (mut allocator, _defer_guard) = get_allocator::<ARENA_SIZE>();

    let ptr = unsafe { allocator.allocate(layout) };

    unsafe {
        allocator
            .deallocate(ptr, layout)
            .expect("Block failed to free");
    }

    let new_ptr = unsafe { allocator.allocate(layout) };
    assert_eq!(ptr, new_ptr);
}
#[test]
fn test_block_reuse_split() {
    let layout = Layout::from_size_align(512, 1).unwrap();

    let (mut allocator, _defer_guard) = get_allocator::<ARENA_SIZE>();

    let ptrs = [
        unsafe { allocator.allocate(layout) },
        unsafe { allocator.allocate(layout) },
        unsafe { allocator.allocate(layout) },
        unsafe { allocator.allocate(layout) },
    ];

    unsafe {
        allocator
            .deallocate(ptrs[1], layout)
            .expect("Block failed to free");
        allocator
            .deallocate(ptrs[2], layout)
            .expect("Block failed to free");
    };

    allocator.defrag();

    alloc_check(ptrs[0], layout, &allocator);
    alloc_check(ptrs[3], layout, &allocator);

    let dealloc_block_1 = allocator
        .find_block_by_ptr(ptrs[1])
        .expect("Unable to find ptr block");
    let dealloc_block_2 = allocator
        .find_block_by_ptr(ptrs[2])
        .expect("Unable to find ptr block");

    assert!(
        dealloc_block_1 == dealloc_block_2,
        "{:#?} != {:#?}",
        dealloc_block_1,
        dealloc_block_2
    )
}

#[test]
fn test_alignment() {
    for i in 1..=12 {
        let layout = Layout::from_size_align(1, 1 << i).unwrap();

        let (mut allocator, _defer_guard) = get_allocator::<ARENA_SIZE>();

        let ptr = unsafe { allocator.allocate(layout) };

        alloc_check(ptr, layout, &allocator);
        assert!(ptr.is_aligned_to(1 << i));

        unsafe {
            allocator
                .deallocate(ptr, layout)
                .expect("Block failed to free");
        }
    }
}

#[test]
fn test_zst() {
    let layout = Layout::from_size_align(0, 1).unwrap();

    let (mut allocator, _defer_guard) = get_allocator::<ARENA_SIZE>();

    let ptr = unsafe { allocator.allocate(layout) };

    assert!(ptr.is_null());
}

#[test]
fn test_box() {
    let value = 32u32;

    let (allocator, _defer_guard) = get_full_allocator::<ARENA_SIZE>();

    let bx = Box::new_in(value, &allocator);
    let ptr = Box::into_raw(bx);

    let blocks = &mut allocator.get().expect("Failed to get allocator");

    assert_eq!(blocks.allocation_balance, 1);
    assert_eq!(unsafe { *ptr }, value);
    alloc_check(ptr, Layout::from_size_align(4, 1).unwrap(), blocks);
    drop(blocks);
    drop(unsafe { Box::from_raw_in(ptr, &allocator) });
}
#[test]
fn test_vec() {
    let (allocator, _defer_guard) = get_full_allocator::<ARENA_SIZE>();
    for i in 0..2000 {
        let mut vec: Vec<u32, _> = Vec::new_in(&allocator);
        let layout = Layout::array::<u8>(100).expect("Failed to create layout");

        for i in 0..1000 {
            vec.push(i);
        }

        let ptr = vec.as_ptr().cast_mut();

        let alloc = allocator.get().expect("Failed to get allocator");

        assert_eq!(alloc.allocation_balance, 1);
        assert_eq!(unsafe { *ptr }, 0);
        alloc_check(ptr, layout, &alloc);
        drop(alloc);
        drop(vec);
        let alloc = allocator.get().expect("Failed to get allocator");
        assert!(!alloc.ptr_is_allocated(ptr as *mut u8));
        assert_eq!(alloc.allocation_balance, 0);
    }
}

#[test]
fn test_large_alloc() {
    for i in 0..5 {
        let layout = Layout::from_size_align(4096, 1).unwrap();

        let (mut allocator, _defer_guard) = get_allocator::<ARENA_SIZE>();

        let ptr = unsafe { allocator.allocate(layout) };

        // This check takes a long time, but it is necessary to ensure that the block is allocated correctly.
        min_check(ptr, layout, &allocator);
        // We don't deallocate the block because A. the test heap is 10 MB and B. we want to test the allocator's ability to handle large allocations.
    }
}

#[test]
fn test_large_alloc_free() {
    for i in 0..15 {
        let layout = Layout::from_size_align(4096, 1).unwrap();

        let (mut allocator, _defer_guard) = get_allocator::<ARENA_SIZE>();

        let ptr = unsafe { allocator.allocate(layout) };

        // This check takes a long time, but it is necessary to ensure that the block is allocated correctly.
        alloc_check(ptr, layout, &allocator);
        if i % 2 == 0 {
            unsafe { allocator.deallocate(ptr, layout) }.expect("Block failed to free");
        }
    }
}
