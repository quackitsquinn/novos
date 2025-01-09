use alloc::vec::Vec;
use x86_64::VirtAddr;

use super::range::VirtualAddressRange;

const MAX_VIRT_ADDR: u64 = 0x0000_7FFF_FFFF_FFFF;
/// The threshold for when to defragment the virtual address space.
const DEFRAG_THRESHOLD: u64 = 0x1000;

pub(super) struct VirtualAddressMapper {
    unused_ranges: Vec<VirtualAddressRange>,
}

impl VirtualAddressMapper {
    pub unsafe fn from_used_ranges(ranges: Vec<VirtualAddressRange>) -> Self {
        let mut unused = Vec::new();
        let mut last = VirtAddr::new(0);
        // TODO: Add 0-nth range but make sure it's not overlapping with the page tables.
        for range in ranges {
            if range.start != last {
                unused.push(VirtualAddressRange::new(
                    last,
                    range.start.as_u64() - last.as_u64(),
                ));
            }
            last = range.end();
        }
        if last.as_u64() < MAX_VIRT_ADDR {
            unused.push(VirtualAddressRange::new(
                last,
                MAX_VIRT_ADDR - last.as_u64(),
            ));
        }
        Self {
            unused_ranges: unused,
        }
    }

    pub fn allocate(&mut self, size: u64) -> Option<VirtualAddressRange> {
        for i in 0..self.unused_ranges.len() {
            if self.unused_ranges[i].size >= size {
                return self.unused_ranges[i].take(size);
            }
        }
        None
    }

    pub fn deallocate(&mut self, range: VirtualAddressRange) {
        // See if the end of the range is equal to the start of any unused ranges.
        let end = range.end();
        for i in 0..self.unused_ranges.len() {
            if self.unused_ranges[i].start == end {
                self.unused_ranges[i].start = range.start;
                self.unused_ranges[i].size += range.size;
                return;
            }
        }
        // If not, add the range to the unused ranges.
        self.unused_ranges.push(range);

        if self.unused_ranges.len() > DEFRAG_THRESHOLD {
            self.defragment();
        }
    }

    fn defragment(&mut self) {
        self.unused_ranges.sort_by_key(|range| range.start);
        let mut i = 0;
        while i < self.unused_ranges.len() - 1 {
            if self.unused_ranges[i].end() == self.unused_ranges[i + 1].start {
                self.unused_ranges[i].size += self.unused_ranges[i + 1].size;
                self.unused_ranges.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
}
