use alloc::{vec, vec::Vec};
use x86_64::VirtAddr;

use super::range::VirtualAddressRange;

/// The threshold for when to defragment the virtual address space.
const DEFRAG_THRESHOLD: usize = 1 << 24;

/// A simple virtual address space allocator.
/// This is specifically for allocating virtual address space, not physical memory. If you dereference any given address you should get a page fault.
#[derive(Debug)]
pub struct VirtualAddressMapper {
    unused_ranges: Vec<VirtualAddressRange>,
}

// This isn't really a speed focused implementation, because it's not really needed.
// Virtual memory isn't going to be needed constantly, and it'll really only be used for the following:
// - Mapping ACPI tables
// - Creating process page tables

impl VirtualAddressMapper {
    /// Create a new virtual address space allocator with the given start and end addresses.
    /// # Safety
    /// The caller must ensure that the given range is valid and not used by anything else.
    pub unsafe fn new(start: VirtAddr, end: VirtAddr) -> Self {
        Self {
            unused_ranges: vec![VirtualAddressRange::new(
                start,
                end.as_u64() - start.as_u64(),
            )],
        }
    }

    /// Allocates the given number of pages, returning the virtual address range.
    /// Returns None if there is not enough space.
    pub fn allocate(&mut self, page_count: u64) -> Option<VirtualAddressRange> {
        let size = page_count * 4096;
        for i in 0..self.unused_ranges.len() {
            if self.unused_ranges[i].size >= size {
                return self.unused_ranges[i].take(size);
            }
        }
        None
    }

    /// Deallocates the given virtual address range.
    /// If the range is already free, this is a no-op.
    pub fn deallocate(&mut self, range: VirtualAddressRange) {
        if self.is_free(range) {
            return;
        }
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

        if self.unused_ranges.len() > DEFRAG_THRESHOLD as usize {
            self.defragment();
        }
    }

    /// Defragments the virtual address space.
    /// Returns the number of passes it took to defragment.
    /// This is a very simple algorithm that just merges adjacent ranges.
    fn defragment(&mut self) -> u64 {
        let mut last_pass = 0;
        while last_pass != 0 {
            // TODO: This might be able to to moved to before the while loop if the algo doesn't clobber the order of the ranges.
            self.unused_ranges.sort_by_key(|range| range.start);
            let mut last = self.unused_ranges[0];
            self.unused_ranges = self
                .unused_ranges
                .iter()
                .skip(1)
                .filter_map(|r| {
                    if last.end() == r.start {
                        last.extend(r.size);
                        last_pass += 1;
                        None
                    } else {
                        let next = last.clone();
                        last = *r;
                        Some(next)
                    }
                })
                .collect();
        }
        last_pass
    }

    fn is_free(&self, range: VirtualAddressRange) -> bool {
        for r in &self.unused_ranges {
            if r.start <= range.start && r.end() >= range.end() {
                return true;
            }
        }
        false
    }
}
