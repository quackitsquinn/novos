use x86_64::{
    PhysAddr,
    structures::paging::{PhysFrame, frame::PhysFrameRangeInclusive},
};

pub mod frame_mapper;

/// A range of physical addresses. This is used to represent a contiguous block of physical memory.
/// Mainly used for physical frame allocation and management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysAddrRange {
    start: PhysAddr,
    end: PhysAddr,
}

impl PhysAddrRange {
    /// Creates a new physical address range with the given start and end addresses.
    /// The start address must be less than the end address.
    pub fn new(start: PhysAddr, end: PhysAddr) -> Self {
        assert!(start < end, "Start address must be less than end address");
        Self { start, end }
    }

    /// Check if this range contains the given physical address.
    pub fn contains(&self, addr: PhysAddr) -> bool {
        addr >= self.start && addr < self.end
    }

    /// The size of this range in bytes.
    pub fn size(&self) -> u64 {
        self.end.as_u64() - self.start.as_u64()
    }

    /// Returns an iterator over the physical frames in this range.
    pub fn to_frame_range(&self) -> PhysFrameRangeInclusive {
        PhysFrame::range_inclusive(
            PhysFrame::containing_address(self.start),
            PhysFrame::containing_address(self.end - 1),
        )
    }

    /// Returns the start address of this range.
    pub fn start(&self) -> PhysAddr {
        self.start
    }

    /// Returns the end address of this range.
    pub fn end(&self) -> PhysAddr {
        self.end
    }
}
