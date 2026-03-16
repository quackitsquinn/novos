use core::ptr::Alignment;

use crate::{
    arch::{self, VirtAddr},
    bitmap::{BitPtr, Bitmap, ENTRY_SIZE},
};

/// The states of the state machine used during allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ScanState {
    /// Scanning for a free range of pages that satisfies the allocation request.
    Scanning,
    // Currently allocating a range larger than `ENTRY_SIZE`, which means we are allocating whole entries in the bitmap.
    // this allows for very fast allocation of large ranges of pages, since we can just check if an entry is zero (fully free) or not, and skip over it if it's not.
    // down the line this can be extended to use SIMD instructions to check multiple entries at once for even faster allocation of large ranges.
    // right now for a 2MiB allocation, it would check and set 8 entries in the bitmap, but 2GiB allocations take 4k entries, which is a lot.
    Allocating {
        start: BitPtr,
        remaining_size: u64,
    },
    // We found memory and do not need to continue scanning.
    Found {
        start: BitPtr,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SkipStrategy {
    /// Check the next entry, don't skip any entries.
    Next,
    /// Skip to the next entry that is aligned for the allocation request.
    NextAligned,
    /// Skip a specific number of entries, which is determined based on the current state and the bitmap entry at the given index.
    N(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct AllocationInfo {
    size: u64,         // The total size of the allocation request in bytes
    align: u64,        // The alignment requirement for the allocation in bytes
    page_align: u64,   // Alignment in pages
    entry_align: u64,  // Alignment in bitmap entries
    needed_pages: u64, // The total number of pages needed for the allocation
}

impl AllocationInfo {
    fn new(size: u64, align: Alignment) -> Self {
        let align = align.as_usize() as u64;
        let page_align = align.div_ceil(arch::TABLE_SIZE);
        let entry_align = page_align.div_ceil(64);
        let needed_pages = bytes_to_pages(size);
        Self {
            size,
            align,
            page_align,
            entry_align,
            needed_pages,
        }
    }
}

pub(super) struct AllocationStateMachine<'a> {
    info: AllocationInfo,
    scan_state: ScanState,
    bitmap: &'a mut Bitmap<'a>,
}

impl<'a> AllocationStateMachine<'a> {
    pub fn new(size: u64, alignment: Alignment, bitmap: &'a mut Bitmap<'a>) -> Self {
        Self {
            info: AllocationInfo::new(size, alignment),
            scan_state: ScanState::Scanning,
            bitmap,
        }
    }

    /// Steps the state machine forward once, checking the bitmap entry at the given index.
    fn do_step(&mut self, index: usize) -> Result<VirtAddr, SkipStrategy> {
        Self::step(self.bitmap, &self.info, &mut self.scan_state, index)
    }

    /// Steps the state machine forward once, checking the bitmap entry at the given index.
    ///
    /// Returns `Ok(addr)` if the allocation is complete. `addr` is a virtual address corresponding to the start of the allocated range.
    ///
    /// Returns `Err(skip)` if the allocation is not yet complete. `skip` is the number of bitmap entries that can be skipped based on the current state and the bitmap entry at the given index,
    /// which can be used to optimize the scanning process.
    fn step(
        bitmap: &mut Bitmap,
        info: &AllocationInfo,
        state_ref: &mut ScanState,
        index: usize,
    ) -> Result<VirtAddr, SkipStrategy> {
        let state = *state_ref;
        let entry = bitmap.data[index];
        match state {
            ScanState::Allocating {
                start,
                remaining_size,
            } if remaining_size > ENTRY_SIZE => {
                // This allocation will take all of this entry, so we just check if it's zero and bump the length remaining
                if entry != 0 {
                    *state_ref = ScanState::Scanning; // Not free, go back to scanning
                    return Err(SkipStrategy::NextAligned); // Skip to the next aligned entry
                }

                *state_ref = ScanState::Allocating {
                    start,
                    remaining_size: remaining_size - ENTRY_SIZE,
                };
                return Err(SkipStrategy::Next); // We don't care about alignment, so just check the next entry
            }
            ScanState::Allocating {
                start,
                remaining_size,
            } => {
                let needed_mask = range_mask(remaining_size);
                if entry & needed_mask != 0 {
                    *state_ref = ScanState::Scanning; // Not enough free pages in this entry, go back to scanning
                    return Err(SkipStrategy::NextAligned); // Skip to the next aligned entry
                }
                // This entry has enough free pages to satisfy the remaining allocation, so we can allocate and return the address
                *state_ref = ScanState::Found { start };
                bitmap.data[index] |= needed_mask; // Mark these pages as allocated in the bitmap
                return Ok(bitmap.addr_for_bitptr(start));
            }
            ScanState::Found { start } => return Ok(bitmap.addr_for_bitptr(start)),
            ScanState::Scanning if entry == u64::MAX => {
                return Err(SkipStrategy::NextAligned); // This entry is fully allocated, so skip to the next aligned entry
            }
            ScanState::Scanning if info.size > ENTRY_SIZE => {
                // We need to allocate more than one entry, so we check if this entry is fully free and properly aligned for the allocation. If so, we can start allocating.
                // TODO: Longer check when len > ENTRY_SIZE but align < ENTRY_SIZE
                if entry != 0 {
                    return Err(SkipStrategy::NextAligned); // Not free, skip to the next aligned entry
                }
                debug_assert!(
                    index % info.entry_align as usize != 0,
                    "Bitmap is not properly aligned for this allocation request"
                );
                *state_ref = ScanState::Allocating {
                    start: BitPtr::new(index, 0),
                    remaining_size: info.size,
                };
                return Err(SkipStrategy::Next); // We just checked this entry, so check the next one
            }
            ScanState::Scanning if info.needed_pages <= info.page_align => {
                let free_mask = !entry; // Invert the entry to get a mask of the free pages
                let range_mask = range_mask(info.needed_pages); // Mask for the number of pages we need to allocate
                let repeated_mask = rep_mask(range_mask, info.page_align); // Create a mask that has the needed number of bits set for the allocation, repeated to fill a u64
                let mask = repeated_mask & free_mask; // Mask of the free bits that also satisfy the alignment
                if mask == 0 {
                    return Err(SkipStrategy::NextAligned); // No suitable free range in this entry, just check the next entry
                }

                for i in 0..((size_of::<u64>() as u64) * 8) / info.page_align {
                    let bit_index = i * info.page_align;
                    let positioned_mask = range_mask << bit_index; // Mask for the current position in the entry
                    if mask & (positioned_mask) != 0 {
                        // This bit is free and properly aligned, so we can allocate here
                        bitmap.data[index] |= positioned_mask; // Mark these pages as allocated in the bitmap
                        let start = BitPtr::new(index, bit_index as usize);
                        *state_ref = ScanState::Found { start };
                        return Ok(bitmap.addr_for_bitptr(start));
                    }
                }
                return Err(SkipStrategy::NextAligned); // We just checked this entry, so check the next one
            }
            ScanState::Scanning => {
                if entry == u64::MAX {
                    return Err(SkipStrategy::NextAligned);
                }

                let needed_mask = range_mask(info.needed_pages); // Mask for the number of pages

                // TODO: something using leading_zeros to find the first free bit
                for bit_index in (0..64).step_by(info.page_align as usize) {
                    let positioned_mask = needed_mask << bit_index; // Mask for the current position in the entry
                    if entry & positioned_mask == 0 {
                        if bit_index + info.needed_pages > 64 {
                            *state_ref = ScanState::Allocating {
                                start: BitPtr::new(index, bit_index as usize),
                                remaining_size: info.size - (64 - bit_index) * arch::TABLE_SIZE,
                            }; // We will allocate the rest of this entry, but we still have remaining size to allocate}
                            return Err(SkipStrategy::Next);
                        }
                        // This range of bits is free, so we can allocate here
                        bitmap.data[index] |= positioned_mask; // Mark these pages as allocated in the bitmap
                        let start = BitPtr::new(index, bit_index as usize);
                        *state_ref = ScanState::Found { start };
                        return Ok(bitmap.addr_for_bitptr(start));
                    }
                }

                return Err(SkipStrategy::NextAligned); // We just checked this entry, so check the next one
            }
        }
    }
}

/// Helper function to create a bitmask for a given length of bits. For example, if `len` is 3, this will return `0b111` (7 in decimal), which can be used to check or set the first 3 bits of a container.
fn range_mask(len: u64) -> u64 {
    if len >= 64 {
        u64::MAX
    } else {
        (1u64 << len) - 1
    }
}
/// Repeats n_bits of mask to fill a u64
///
/// n_bits must be a power of two and less than or equal to 64
fn rep_mask(mask: u64, n_bits: u64) -> u64 {
    assert!(
        n_bits <= 64 && n_bits.is_power_of_two(),
        "n_bits must be a power of two and less than or equal to 64"
    );

    if n_bits == 64 {
        return mask;
    }

    let mask = mask & (1 << n_bits) - 1; // Ensure mask is only n_bits long
    let mut result = 0;
    for i in 0..(64 / n_bits) {
        result |= (mask as u64) << (i * n_bits); // Shift the mask to the correct position and combine it with the result
    }
    result
}

/// Helper function to calculate how many pages are needed to cover a given length in bytes.
/// This is used to determine how many bits need to be allocated in the bitmap for a given allocation request.
///
///
fn bytes_to_pages(len: u64) -> u64 {
    len.div_ceil(arch::TABLE_SIZE)
}

#[cfg(test)]
mod tests {
    use std::ptr::Alignment;

    use crate::{
        arch::{self, VirtAddr},
        bitmap::{
            BitPtr, Bitmap,
            state::{AllocationInfo, AllocationStateMachine, ScanState, SkipStrategy},
            tests::OwnedBitmap,
        },
    };

    /// Tests for the various unit conversion / helper functions in Bitmap.
    ///
    /// Testing your low-level helper functions is crucial for ensuring correctness and preventing nasty bugs.
    /// Any of these tests failing implicitly indicates that any other failing tests are likely due to the failure of these helper functions.
    mod math {
        use crate::{
            arch,
            bitmap::state::{bytes_to_pages, range_mask, rep_mask},
        };

        use super::*;

        #[test]
        fn test_range_mask() {
            assert_eq!(range_mask(0), 0);
            assert_eq!(range_mask(1), 0b1);
            assert_eq!(range_mask(2), 0b11);
            assert_eq!(range_mask(3), 0b111);
            assert_eq!(range_mask(64), u64::MAX);
            assert_eq!(range_mask(888), u64::MAX);
        }

        #[test]
        fn test_rep_mask() {
            assert_eq!(rep_mask(0b1, 1), 0xFFFF_FFFF_FFFF_FFFF);
            assert_eq!(rep_mask(0b1, 2), 0x5555_5555_5555_5555);
            assert_eq!(rep_mask(0b11, 2), 0xFFFF_FFFF_FFFF_FFFF);
            assert_eq!(rep_mask(0xDEADBEEF, 32), 0xDEADBEEF_DEADBEEF);
        }

        #[test]
        fn test_bytes_to_pages() {
            assert_eq!(bytes_to_pages(0), 0);
            assert_eq!(bytes_to_pages(arch::TABLE_SIZE - 1), 1);
            assert_eq!(bytes_to_pages(arch::TABLE_SIZE), 1);
            assert_eq!(bytes_to_pages(arch::TABLE_SIZE + 1), 2);
        }
    }

    const TEST_BASE: VirtAddr = VirtAddr::new(0x1000_0000);

    fn state_machine<'a>(
        bitmap: &'a mut Bitmap<'a>,
        size: u64,
        alignment: usize,
    ) -> AllocationStateMachine<'a> {
        AllocationStateMachine::new(
            arch::TABLE_SIZE,
            Alignment::new(alignment).expect("state_machine: alignment not power of 2"),
            bitmap,
        )
    }

    #[test]
    fn test_alloc_one_unalign() {
        let mut owner = OwnedBitmap::new(4, TEST_BASE);
        let mut bitmap = owner.bitmap();
        let mut state_machine = state_machine(bitmap, arch::TABLE_SIZE, 1);

        let res = state_machine.do_step(0);

        assert_eq!(res, Ok(TEST_BASE));
        assert_eq!(state_machine.bitmap.data[0], 0b1);
        assert_eq!(
            state_machine.scan_state,
            ScanState::Found {
                start: BitPtr::new(0, 0)
            }
        );
    }

    #[test]
    fn test_alloc_one_entry_aligned_skip() {
        let mut owner = OwnedBitmap::new(4, TEST_BASE);
        let mut bitmap = owner.bitmap();
        let mut state_machine =
            state_machine(bitmap, arch::TABLE_SIZE, arch::TABLE_SIZE as usize * 64);

        let res = state_machine.do_step(0);

        assert_eq!(res, Ok(TEST_BASE));
        assert_eq!(
            state_machine.scan_state,
            ScanState::Found {
                start: BitPtr::new(0, 0)
            }
        );
        assert_eq!(state_machine.bitmap.data[0], 0b1);

        state_machine.scan_state = ScanState::Scanning; // Reset the state machine to scanning to test the skip strategy
        let res = state_machine.do_step(0);
        assert_eq!(res, Err(SkipStrategy::NextAligned));
        let res = state_machine.do_step(1);
        assert_eq!(res, Ok(TEST_BASE + (arch::TABLE_SIZE as u64 * 64)));
    }
}
