use core::fmt::Alignment;

use crate::{
    bitmap::{Bitmap, VirtualMemoryManager},
    entry_walker::EntryWalker,
};

pub struct PhysicalMemoryManager {
    bitmaps: &'static [BitmapEntry],
}

impl PhysicalMemoryManager {
    pub unsafe fn init(_entry_walker: &mut EntryWalker, _vmm: &mut VirtualMemoryManager) -> Self {
        todo!(
            "pending some sort of page based allocation primitives, not entirely sure how it would work."
        )
    }
}

struct BitmapEntry {
    // contains the base address of the range
    bitmap: &'static mut Bitmap<'static>,
    /// the max alignment that this bitmap can guarantee for it's allocations.
    ///
    /// there will be a way to configure how many entries in the manager that are aligned to higher alignments,
    /// which removes any alignment logic from the bitmap itself. ideally there will be ranges that do map well to higher alignments, but
    /// we can just align up to the next alignment boundary, which yes, does waste some memory, but it is simpler and removes the weird alignment logic that a unaligned
    /// bitmap would require.
    alignment: Alignment,
    /// the total number of free pages in this bitmap. used to skip bitmaps that are full.
    free: u64,
}
