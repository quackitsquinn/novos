use alloc::{vec, vec::Vec};
use x86_64::VirtAddr;

use super::range::VirtualAddressRange;

const MAX_VIRT_ADDR: u64 = 0x0000_7FFF_FFFF_FFFF;
/// The threshold for when to defragment the virtual address space.
const DEFRAG_THRESHOLD: usize = 1 << 24;

pub struct VirtualAddressMapper {
    unused_ranges: Vec<VirtualAddressRange>,
}

// This isn't really a speed focused implementation, because it's not really needed.
// Virtual memory isn't going to be needed constantly, and it'll really only be used for the following:
// - Mapping ACPI tables
// - Creating process page tables

impl VirtualAddressMapper {
    pub unsafe fn new(start: VirtAddr, end: VirtAddr) -> Self {
        Self {
            unused_ranges: vec![VirtualAddressRange::new(
                start,
                end.as_u64() - start.as_u64(),
            )],
        }
    }
    #[deprecated]
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
    #[deprecated]
    pub unsafe fn from_unused_ranges(ranges: Vec<VirtualAddressRange>) -> Self {
        Self {
            unused_ranges: ranges,
        }
    }
    // FIXME: Refactor into page_count rather than size.
    // The current implementation will massively break if the size is not a multiple of 4096, which should probably be a guarantee.
    pub fn allocate(&mut self, size: u64) -> Option<VirtualAddressRange> {
        for i in 0..self.unused_ranges.len() {
            if self.unused_ranges[i].size >= size {
                return self.unused_ranges[i].take(size);
            }
        }
        None
    }

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

    fn defragment(&mut self) -> u64 {
        let mut last_pass = 0;
        while last_pass != 0 {
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

//#[cfg(test)]
mod tests {
    use alloc::vec;
    use kproc::test;
    use x86_64::VirtAddr;

    use crate::memory::paging::virt::VirtualAddressRange;

    #[test("VAM allocate", can_recover = true)]
    pub fn test_vam_allocate() {
        let ranges = vec![
            VirtualAddressRange::new_page(VirtAddr::new(0)),
            VirtualAddressRange::new_page(VirtAddr::new(4096)),
            VirtualAddressRange::new_page(VirtAddr::new(8192)),
        ];
        let mut vam = unsafe { super::VirtualAddressMapper::from_used_ranges(ranges) };
        let range = vam.allocate(4096).unwrap();
        assert_eq!(range.start, VirtAddr::new(12288));
        assert_eq!(range.size, 4096);
        assert!(!vam.is_free(range))
    }

    #[test("VAM deallocate", can_recover = true)]
    pub fn test_vam_deallocate() {
        let ranges = vec![
            VirtualAddressRange::new_page(VirtAddr::new(0)),
            VirtualAddressRange::new_page(VirtAddr::new(4096)),
            VirtualAddressRange::new_page(VirtAddr::new(8192)),
        ];

        let mut vam = unsafe { super::VirtualAddressMapper::from_used_ranges(ranges) };

        let range = VirtualAddressRange::new_page(VirtAddr::new(12288));

        vam.deallocate(range);
    }
}
