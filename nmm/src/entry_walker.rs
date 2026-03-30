//! A helper struct for iterating over memory map entries and calculating the total usable memory.
use core::fmt::Debug;
use core::ptr::Alignment;
use core::{mem, slice};

use arrayvec::ArrayVec;
use cake::limine::memory_map::{Entry, EntryType};

use crate::arch::PhysAddr;
use crate::paging::limine::LimineEntry;

/// A helper struct for iterating over memory map entries and calculating the total usable memory.
#[allow(missing_debug_implementations)] // TODO: allowed to silence the warning for now
pub struct EntryWalker<'a> {
    pub entries: &'a [&'a LimineEntry],
    idx: usize,
    current: Option<LimineEntry>,
    // Contains entries that were skipped either due to alignment requirements or because they were too small, but may still be usable for smaller allocations
    // TODO: tweak CAP
    extra_entries: ArrayVec<ExtraEntry, 0x88>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ExtraEntry {
    base: u64,
    length: u64,
}

impl From<LimineEntry> for ExtraEntry {
    fn from(entry: LimineEntry) -> Self {
        // SAFETY: ExtraEntry has the same memory layout as the first 2 fields of Entry, so we can transmute it without any issues.
        // We only need the base and length fields for the extra entries, since the entry type is not relevant for them.
        unsafe { mem::transmute_copy(&entry) }
    }
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
            entries: unsafe {
                // SAFETY: LimineEntry has the same memory layout as Entry.
                slice::from_raw_parts(entries.as_ptr().cast(), entries.len())
            },
            idx: 0,
            current: None,
            extra_entries: ArrayVec::new(),
        }
    }

    /// Creates a new `EntryWalker` with the given slice of Limine memory map entries.
    #[cfg(test)]
    pub(crate) fn from_limine_entries(entries: &'a [&'a LimineEntry]) -> Self {
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
            #[cfg(test)]
            eprintln!(
                "early return 0: no entries or first entry too small:  {} || {}",
                self.extra_entries.is_empty(),
                match self.extra_entries.first() {
                    Some(entry) => format!("entry length {:#x} < len {:#x}", entry.length, len),
                    None => "no entries".to_string(),
                }
            );
            return None;
        }

        // Aligns the given addr up to a multiple of the specified alignment. This is used to ensure that the returned physical address meets the alignment requirements for the allocation.
        let align_up = |addr: u64| align_up(addr, alignment);

        for i in 0..self.extra_entries.len() {
            let entry = &mut self.extra_entries[i];
            if entry.length < len {
                #[cfg(test)]
                eprintln!(
                    "early return 1: entry {} too small: {} < {}",
                    i, entry.length, len
                );
                // The array is guaranteed to be sorted by length in descending order, so if the first entry is too small, all entries are too small
                return None;
            }

            let aligned_base = align_up(entry.base);
            let end = entry.base + entry.length;
            if aligned_base + len > end {
                #[cfg(test)]
                eprintln!(
                    "continue: entry {} can't fit allocation with alignment: aligned base {:#x} + len {:#x} >= end {:#x}",
                    i, aligned_base, len, end
                );
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
                self.extra_entries
                    .sort_unstable_by_key(|v| u64::MAX - v.length);
            }

            return Some(PhysAddr::new(allocated_base));
        }

        // The inversion will swap the order into descending order, so the longest entries will be at ind 0.
        self.extra_entries
            .sort_unstable_by_key(|v| u64::MAX - v.length);

        #[cfg(test)]
        eprintln!(
            "late return: no suitable entry found for {:#x}:{:#x}\nextra entries: {:?}",
            len,
            alignment.as_usize(),
            self.extra_entries
        );

        None
    }

    /// Returns the next usable memory map entry, or `None` if there are no more usable entries.
    /// This method iterates through the memory map entries, skipping non-usable entries and returning each usable entry until all entries have been processed.
    ///
    /// This will overwrite `self.current` with the next entry.
    fn next_entry(&mut self) -> Option<LimineEntry> {
        for i in self.idx..self.entries.len() {
            let entry = *self.entries[i];
            self.idx = i + 1;
            if matches!(entry.entry_type, EntryType::USABLE) {
                self.current = Some(entry);
                return Some(entry);
            }
        }
        None
    }

    /// Returns the current entry if there is one, or the next usable entry if there isn't.
    fn get_entry(&mut self) -> Option<LimineEntry> {
        if let Some(entry) = self.current {
            Some(entry)
        } else {
            self.next_entry()
        }
    }

    /// Returns the next usable physical address, or `None` if there are no more usable entries.
    /// This method iterates through the memory map entries, skipping non-usable entries and returning the starting physical address of each usable entry until all entries have been processed.
    //
    pub fn next(&mut self, len: u64, alignment: Alignment) -> Option<PhysAddr> {
        // First, if there are any extra entries, try to take from them first
        if let Some(addr) = self.try_take_reserved(len, alignment) {
            return Some(addr);
        }

        // If there are no extra entries, we need to get
        let mut current_entry = self.get_entry()?;
        let end = current_entry.base + current_entry.length;
        let aligned_base = align_up(current_entry.base, alignment);
        let aligned_end = aligned_base + len;

        if current_entry.length < len || aligned_end > end {
            // This entry is too small, so we skip it and try the next one
            self.extra_entries.push(current_entry.into());
            self.current = None;
            return self.next(len, alignment);
        }

        if aligned_base > current_entry.base {
            // If there is a gap between the original base and the aligned base, we can add an extra entry for that gap
            self.extra_entries.push(current_entry.into());
        }

        current_entry.base += len;
        current_entry.length -= len;

        self.current = Some(current_entry);

        return Some(PhysAddr::new(aligned_base));
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
        let mut walker = EntryWalker::from_limine_entries(entries);
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

    fn make_entry(base: u64, length: u64, entry_type: EntryType) -> LimineEntry {
        LimineEntry {
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
        #[rustfmt::skip]
        let entries = &[
            extra_entry(0x1000, 0x1000), 
            extra_entry(0x3000, 0x1000)
        ];

        let mut walker = EntryWalker {
            entries: &[],
            idx: 0,
            current: None,
            extra_entries: ArrayVec::try_from(&entries[..]).expect("Failed to create ArrayVec"),
        };

        // Take half of the first entry, which should succeed and leave an extra entry with the remaining half
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x1000).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x1000)));
        // Attempt to take the second half, but the high alignment forces it to take the second entry instead.
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x1000).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x3000)));
        // Try to take another 0x1000 aligned chunk of memory, which will fail since the remaining halves of both entries are only 0x800 in size, and the alignment requirements
        // can't be met
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x1000).unwrap());
        assert_eq!(addr, None);
        // Take the remaining half of the first entry, which should succeed and remove the entry from the list since it's now empty
        let addr = walker.try_take_reserved(0x800, Alignment::new(0x800).unwrap());
        assert_eq!(addr, Some(PhysAddr::new(0x1800)));

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

    #[test]
    fn test_next_entry() {
        let entries = &[
            &make_entry(0x0, 0x0F, EntryType::RESERVED),
            &make_entry(0x10, 0x10, EntryType::USABLE),
            &make_entry(0x20, 0xF0, EntryType::RESERVED),
            &make_entry(0x40, 0xF00, EntryType::BOOTLOADER_RECLAIMABLE),
        ];
        let mut walker = EntryWalker::from_limine_entries(entries);
        assert_eq!(walker.next_entry(), Some(*entries[1]));
        assert_eq!(walker.next_entry(), None);
        if walker.next_entry().is_some() {
            panic!("Expected no more entries");
        }
    }

    #[test]
    fn test_next_skip_reserved() {
        let entries = &[
            &make_entry(0x0, 0x20, EntryType::RESERVED),
            &make_entry(0x20, 0x20, EntryType::ACPI_NVS),
            &make_entry(0x40, 0x20, EntryType::ACPI_RECLAIMABLE),
            &make_entry(0x60, 0x20, EntryType::FRAMEBUFFER),
            &make_entry(0x80, 0x20, EntryType::EXECUTABLE_AND_MODULES),
            &make_entry(0xA0, 0x20, EntryType::BAD_MEMORY),
            &make_entry(0xC0, 0x20, EntryType::BOOTLOADER_RECLAIMABLE),
            &make_entry(0xE0, 0x20, EntryType::USABLE),
        ];

        let mut walker = EntryWalker::from_limine_entries(entries);
        assert_eq!(
            walker.next(0x10, Alignment::new(0x10).unwrap()),
            Some(PhysAddr::new(0xE0))
        );
    }

    #[test]
    fn test_next_simple() {
        let entries = &[
            &make_entry(0x0, 0x20, EntryType::RESERVED),
            &make_entry(0x20, 0x20, EntryType::USABLE),
        ];

        let mut walker = EntryWalker::from_limine_entries(entries);
        assert_eq!(
            walker.next(0x10, Alignment::new(0x10).unwrap()),
            Some(PhysAddr::new(0x20))
        );
    }

    #[test]
    fn test_next_alignment() {
        let entries = &[&make_entry(0x1, 0x20, EntryType::USABLE)];

        let mut walker = EntryWalker::from_limine_entries(entries);
        assert_eq!(
            walker.next(0x10, Alignment::new(0x10).unwrap()),
            Some(PhysAddr::new(0x10))
        );
        assert!(!walker.extra_entries.is_empty(),);
        assert_eq!(
            walker.next(0xF, Alignment::new(0x1).unwrap()),
            Some(PhysAddr::new(0x1)),
        );
    }
}
