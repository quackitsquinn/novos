use arrayvec::ArrayVec;
use log::warn;
use x86_64::structures::paging::{
    FrameAllocator, FrameDeallocator, frame::PhysFrameRangeInclusive,
};

use crate::{KernelPageSize, KernelPhysFrame, phys::PhysAddrRange};

const UNMAPPED_PAGE_CAPACITY: usize = 0x1000;

pub struct FrameMapper<T>
where
    T: Iterator<Item = PhysAddrRange>,
{
    ranges: T,
    current: PhysAddrRange,
    current_range: PhysFrameRangeInclusive,
    // TODO: Make this a vec of PhysAddrRange because it would be a lot more efficient
    //       to store unmapped frames as ranges instead of individual frames.
    // TODO: Create some sort of collection trait that only has `push` and `pop` methods
    //       so that this can use an actual vector if feature = "alloc", and an arrayvec if not.
    unmapped_frames: ArrayVec<KernelPhysFrame, UNMAPPED_PAGE_CAPACITY>,
}

impl<T> FrameMapper<T>
where
    T: Iterator<Item = PhysAddrRange>,
{
    pub fn new(mut ranges: T) -> Self {
        let current = ranges.next().expect("No ranges provided");
        let current_range = current.to_frame_range();
        Self {
            ranges,
            current,
            current_range,
            unmapped_frames: ArrayVec::new(),
        }
    }

    pub fn next_frame(&mut self) -> Option<KernelPhysFrame> {
        // If we have unmapped frames, return one of them
        if let Some(frame) = self.unmapped_frames.pop() {
            return Some(frame);
        }
        // If we have frames in the current range, return the next one
        if let Some(frame) = self.current_range.next() {
            return Some(frame);
        }

        // If we reach here, we need to get the next range
        if let Some(next_range) = self.ranges.next() {
            self.current = next_range;
            self.current_range = next_range.to_frame_range();
            if let Some(frame) = self.current_range.next() {
                return Some(frame);
            }
        }

        None
    }
}

unsafe impl<T> FrameAllocator<KernelPageSize> for FrameMapper<T>
where
    T: Iterator<Item = PhysAddrRange>,
{
    fn allocate_frame(&mut self) -> Option<KernelPhysFrame> {
        self.next_frame()
    }
}

impl<T> FrameDeallocator<KernelPageSize> for FrameMapper<T>
where
    T: Iterator<Item = PhysAddrRange>,
{
    unsafe fn deallocate_frame(&mut self, frame: KernelPhysFrame) {
        if self.unmapped_frames.len() < UNMAPPED_PAGE_CAPACITY {
            self.unmapped_frames.push(frame);
        } else {
            warn!(
                "FrameMapper: Unmapped frame buffer is full, unable to deallocate: {:?}",
                frame
            );
        }
    }
}
