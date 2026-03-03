use core::fmt::Debug;
use core::{alloc::Layout, ptr::Alignment};

use arrayvec::ArrayVec;
use cake::limine::memory_map::{Entry, EntryType};

use crate::arch::PhysAddr;

/// A helper struct for iterating over memory map entries and calculating the total usable memory.
#[allow(missing_debug_implementations)] // TODO: allowed to silence the warning for now
pub struct EntryWalker<'a> {
    entries: &'a [&'a Entry],
    idx: usize,
    current: Option<(Entry, u64)>,
    // Contains entries that were skipped either due to alignment requirements or because they were too small, but may still be usable for smaller allocations
    // TODO: tweak CAP
    extra_entries: ArrayVec<ExtraEntry, 0x88>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ExtraEntry {
    base: u64,
    length: u64,
}

impl Debug for ExtraEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExtraEntry")
            .field("base", &format_args!("{:#x}", self.base))
            .field("length", &format_args!("{:#x}", self.length))
            .finish()
    }
}

impl<'a> EntryWalker<'a> {
    /// Creates a new `EntryWalker` with the given slice of memory map entries.
    pub fn new(entries: &'a [&'a Entry]) -> Self {
        Self {
            entries,
            idx: 0,
            current: None,
            extra_entries: ArrayVec::new(),
        }
    }

    /// Returns the amount of usable memory in bytes.
    pub fn usable_memory(&mut self) -> u64 {
        let mut total = 0;
        for entry in self.entries {
            if matches!(
                entry.entry_type,
                EntryType::USABLE | EntryType::BOOTLOADER_RECLAIMABLE | EntryType::ACPI_RECLAIMABLE
            ) {
                total += entry.length;
            }
        }
        total
    }

    /// Attempts to take a chunk of memory of the specified length and alignment from the extra entries,
    /// returning the base physical address of the allocated chunk if successful.
    ///
    /// If a suitable entry is found, it updates the entry to reflect the allocated portion and returns the base physical address of the allocated chunk.
    /// If no suitable entry is found, it returns `None`.
    fn try_take_reserved(&mut self, len: u64, alignment: Alignment) -> Option<PhysAddr> {
        if self.extra_entries.is_empty() || self.extra_entries[0].length < len {
            return None;
        }

        // Aligns the given addr up to a multiple of the specified alignment. This is used to ensure that the returned physical address meets the alignment requirements for the allocation.
        let align_up = |addr: u64| align_up(addr, alignment);

        for i in 0..self.extra_entries.len() {
            let entry = &mut self.extra_entries[i];
            if entry.length < len {
                // The array is guaranteed to be sorted by length in descending order, so if the first entry is too small, all entries are too small
                return None;
            }

            let aligned_base = align_up(entry.base);
            let end = entry.base + entry.length;
            if aligned_base + len >= end {
                // TODO: might be able to just return None here?
                continue;
            }

            // This entry can fit the allocation, so we take it and update the entry to reflect the allocated portion
            let allocated_base = aligned_base;
            let allocated_end = allocated_base + len;
            let old_base = entry.base;
            entry.base = allocated_end;
            entry.length = end - allocated_end;
            let entry = *entry; // Copy the entry so we can drop the mutable borrow before we potentially insert a new extra entry for the gap between the original base and the aligned base, since that would require a mutable borrow of self.extra_entries
            if entry.length == 0 {
                // If the entry is now empty, we can remove it from the list
                self.extra_entries.remove(i);
            }
            if aligned_base > old_base {
                // If there is a gap between the original base and the aligned base, we can add an extra entry for that gap
                self.extra_entries.push(ExtraEntry {
                    base: old_base,
                    length: aligned_base - old_base,
                });
            }
            return Some(PhysAddr::new(allocated_base));
        }

        // The inversion will swap the order into descending order, so the longest entries will be at ind 0.
        self.extra_entries
            .sort_unstable_by_key(|v| u64::MAX - v.length);

        None
    }

    /// Returns the next usable physical address, or `None` if there are no more usable entries.
    /// This method iterates through the memory map entries, skipping non-usable entries and returning the starting physical address of each usable entry until all entries have been processed.
    //
    pub fn next(&mut self, len: u64, alignment: Alignment) -> Option<PhysAddr> {
        // First, if there are any extra entries, try to take from them first
        if let Some(addr) = self.try_take_reserved(len, alignment) {
            return Some(addr);
        }

        todo!()
    }
}

fn align_up(addr: u64, alignment: Alignment) -> u64 {
    let align = alignment.as_usize() as u64;
    (addr + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {

    use std::{array, ptr::Alignment};

    use cake::limine::memory_map::{Entry, EntryType};

    use super::*;

    #[test]
    fn test_usable_memory() {
        let entries = &[
            &make_entry(0x0, 0xF, EntryType::USABLE),
            &make_entry(0x10, 0x10, EntryType::RESERVED),
            &make_entry(0x20, 0xF0, EntryType::BOOTLOADER_RECLAIMABLE),
            &make_entry(0x40, 0xF00, EntryType::ACPI_RECLAIMABLE),
        ];
        let mut walker = super::EntryWalker::new(entries);
        assert_eq!(walker.usable_memory(), 0xFFF);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(
            super::align_up(0x1000, Alignment::new(0x1000).unwrap()),
            0x1000
        );
        assert_eq!(
            super::align_up(0x1001, Alignment::new(0x1000).unwrap()),
            0x2000
        );
        assert_eq!(
            super::align_up(0x1FFF, Alignment::new(0x1000).unwrap()),
            0x2000
        );
        assert_eq!(
            super::align_up(0x2000, Alignment::new(0x1000).unwrap()),
            0x2000
        );
    }

    fn make_entry(base: u64, length: u64, entry_type: EntryType) -> Entry {
        Entry {
            base,
            length,
            entry_type,
        }
    }

    fn extra_entry(base: u64, length: u64) -> ExtraEntry {
        ExtraEntry { base, length }
    }

    #[test]
    fn test_try_take_reserved() {
        let entries = &[extra_entry(0x1000, 0x1000), extra_entry(0x3000, 0x1000)];

        let mut walker = EntryWalker {
            entries: &[],
            idx: 0,
            current: None,
            extra_entries: ArrayVec::try_from(&entries[..]).expect("Failed to create ArrayVec"),
        };
        // Take half of the first entry, which should succeed and leave an extra entry with the remaining half
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x1000).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x1000)));
        // Take the first half of the second entry, skipping the first entry since it's alignment requirements can't be met,
        // which should succeed and leave an extra entry with the remaining half
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x1000).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x3000)));
        // Try to take another 0x1000 aligned chunk of memory, which will fail since the remaining halves of both entries are only 0x800 in size, and the alignment requirements
        // can't be met
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x1000).unwrap());
        assert_eq!(addr, None);
        // Take the remaining half of the first entry, which should succeed and remove the entry from the list since it's now empty
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x800).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x1800)));
        assert_eq!(walker.extra_entries.len(), 1,);
        // Finally, take the remaining half of the second entry, which should succeed and remove the entry from the list since it's now empty
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x800).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x3800)));
        assert!(walker.extra_entries.is_empty());
    }

    #[test]
    fn test_try_take_reserved_no_align_leak() {
        let entries = &[
            extra_entry(0x0, 0x400000), // A 4 MiB entry
        ];

        let mut walker = EntryWalker {
            entries: &[],
            idx: 0,
            current: None,
            extra_entries: ArrayVec::try_from(&entries[..]).expect("Failed to create ArrayVec"),
        };

        // Take a 4kib chunk first, which should succeed and leave an extra entry with the remaining 4 MiB - 4 KiB
        let addr = walker.try_take_reserved(0x1000, Alignment::new(0x1000).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x0)));
        assert_eq!(walker.extra_entries.len(), 1);

        // Take a 2Mib aligned 2Mib chunk, which should be offset by 2Mib for alignment
        let addr = walker.try_take_reserved(0x200000, Alignment::new(0x200000).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x200000)));
        assert_eq!(walker.extra_entries.len(), 1);

        // We should be able to take another 511 4kib chunks, ensuring that the alignment requirements for the 2Mib chunk didn't cause us to lose any usable memory,
        // since the original entry was large enough to accommodate both the 2Mib chunk and the 511 4kib chunks even with the alignment requirements
        for i in 0..511 {
            let addr = walker.try_take_reserved(0x1000, Alignment::new(0x1000).unwrap());
            assert_eq!(addr, Some(PhysAddr::new(0x1000 * (i + 1))));
            if i != 510 {
                assert_eq!(walker.extra_entries.len(), 1, "{}", i);
            } else {
                assert!(walker.extra_entries.is_empty());
            }
        }
    }
}
