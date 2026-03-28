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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SkipStrategy {
    /// Check the next entry, don't skip any entries.
    Next,
    /// Skip to the next entry that is aligned for the allocation request.
    NextAligned,
    /// Skip a specific number of entries, which is determined based on the current state and the bitmap entry at the given index.
    #[allow(dead_code)] // for now..
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
    fn do_step(&mut self, index: u64) -> Result<VirtAddr, SkipStrategy> {
        Self::step(self.bitmap, &self.info, &mut self.scan_state, index)
            .map(|f| self.bitmap.addr_for_bitptr(f))
    }

    pub fn run(&mut self) -> Option<VirtAddr> {
        let mut index = 0;
        while index < self.bitmap.data.len() as u64 {
            match self.do_step(index) {
                Ok(addr) => return Some(addr),
                Err(SkipStrategy::Next) => index += 1,
                Err(SkipStrategy::NextAligned) => {
                    let align_mask = self.info.entry_align - 1;
                    index = (index + self.info.entry_align) & !(align_mask);
                }
                Err(SkipStrategy::N(n)) => index += n,
            }
        }
        None
    }

    fn handle_allocation(
        start: BitPtr,
        remaining_size: u64,
        bitmap: &mut Bitmap,
        state_ref: &mut ScanState,
        index: u64,
    ) -> Result<BitPtr, SkipStrategy> {
        let entry = bitmap.data[index as usize];

        if remaining_size > ENTRY_SIZE {
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

        let needed_mask = range_mask(bytes_to_pages(remaining_size));
        if entry & needed_mask != 0 {
            *state_ref = ScanState::Scanning; // Not enough free pages in this entry, go back to scanning
            return Err(SkipStrategy::NextAligned); // Skip to the next aligned entry
        }

        Ok(start)
    }

    fn scan(
        bitmap: &mut Bitmap,
        info: &AllocationInfo,
        state_ref: &mut ScanState,
        index: u64,
    ) -> Result<BitPtr, SkipStrategy> {
        let entry = bitmap.data[index as usize];
        if entry == u64::MAX {
            return Err(SkipStrategy::NextAligned); // This entry is fully allocated, so skip to the next aligned entry
        }

        if info.size > ENTRY_SIZE && info.align >= arch::TABLE_SIZE {
            // We need to allocate more than one entry, so we check if this entry is fully free and properly aligned for the allocation. If so, we can start allocating.
            // TODO: Longer check when len > ENTRY_SIZE but align < ENTRY_SIZE
            if entry != 0 {
                return Err(SkipStrategy::NextAligned); // Not free, skip to the next aligned entry
            }
            debug_assert!(
                bitmap.addr_for_index(index).as_u64() % info.align == 0,
                "Bitmap is not properly aligned for this allocation request"
            );
            *state_ref = ScanState::Allocating {
                start: BitPtr::new(index, 0),
                remaining_size: info.size - ENTRY_SIZE,
            };

            return Err(SkipStrategy::Next); // We just checked this entry, so check the next one
        }

        if info.needed_pages <= info.page_align && info.page_align <= 64 {
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
                    let start_val = BitPtr::new(index, bit_index as u8);
                    return Ok(start_val);
                }
            }

            return Err(SkipStrategy::NextAligned); // We just checked this entry, so check the next one
        }

        let needed_mask = range_mask(info.needed_pages); // Mask for the number of pages

        // TODO: something using leading_zeros to find the first free bit
        for bit_index in (0..64).step_by(info.page_align as usize) {
            let positioned_mask = needed_mask << bit_index; // Mask for the current position in the entry
            if entry & positioned_mask == 0 {
                if bit_index + info.needed_pages > 64 {
                    *state_ref = ScanState::Allocating {
                        start: BitPtr::new(index, bit_index as u8),
                        remaining_size: info.size - (64 - bit_index) * arch::TABLE_SIZE,
                    }; // We will allocate the rest of this entry, but we still have remaining size to allocate}
                    return Err(SkipStrategy::Next);
                }
                return Ok(BitPtr::new(index, bit_index as u8));
            }
        }

        return Err(SkipStrategy::NextAligned); // We just checked this entry, so check the next one
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
        index: u64,
    ) -> Result<BitPtr, SkipStrategy> {
        let state = *state_ref;

        #[cfg(test)]
        println!(
            "index: {}, entry: {:b}, state: {:?}, info: {:?}",
            index, bitmap.data[index as usize], state, info
        );

        let res: BitPtr = match state {
            ScanState::Allocating {
                start,
                remaining_size,
            } => Self::handle_allocation(start, remaining_size, bitmap, state_ref, index)?,

            ScanState::Scanning => Self::scan(bitmap, info, state_ref, index)?,
        };

        // While it probably won't be used in the implementation (maybe), we want to retain the ability to use a single state machine to allocate
        // multiple chunks of memory with the same layout requirements. The reasoning is speed, but I've yet to test if it presents a significant
        // performance improvement over the current create as needed approach. TODO: more data needed
        *state_ref = ScanState::Scanning;

        unsafe { bitmap.set(res, info.needed_pages) }; // Mark the allocated pages in the bitmap
        Ok(res)
    }
}

/// Helper function to create a bitmask for a given length of bits.
/// For example, if `len` is 3, this will return `0b111` (7 in decimal), which can be used to check or set the first 3 bits of a container.
///
/// If `len > 64`, this will return `u64::MAX`.
pub(crate) fn range_mask(len: u64) -> u64 {
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

    let mask = mask & ((1 << n_bits) - 1); // Ensure mask is only n_bits long
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
            state::{
                AllocationInfo, AllocationStateMachine, ScanState, SkipStrategy, range_mask,
                rep_mask,
            },
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
            size,
            Alignment::new(alignment).expect("state_machine: alignment not power of 2"),
            bitmap,
        )
    }

    fn validate_allocation(
        bitmap: &Bitmap,
        addr: VirtAddr,
        expected: VirtAddr,
        size: u64,
        alignment: usize,
    ) {
        assert!(
            addr.as_u64() % alignment as u64 == 0,
            "Address is not properly aligned"
        );
        assert!(
            addr.as_u64() >= bitmap.base.as_u64(),
            "Address is below bitmap base"
        );
        assert!(
            addr.as_u64() + size <= bitmap.base.as_u64() + bitmap.page_count * arch::TABLE_SIZE,
            "Address range exceeds bitmap bounds"
        );
        assert!(size > 0, "Size must be greater than 0");
        assert_eq!(
            addr, expected,
            "Allocated address does not match expected address"
        );
        let offset = addr.as_u64() - bitmap.base.as_u64();
        assert!(
            bitmap
                .is_set(
                    bitmap.bitptr_for_addr(addr).expect("addr invalid bitptr"),
                    size / arch::TABLE_SIZE
                )
                .expect("ptr should not be out of range")
        )
    }

    #[test]
    fn test_alloc_one_unalign() {
        let mut owner = OwnedBitmap::new(4, TEST_BASE);
        let mut bitmap = owner.bitmap();
        let mut state_machine = state_machine(bitmap, arch::TABLE_SIZE, 1);

        let res = state_machine.do_step(0).expect("should not skip");

        validate_allocation(state_machine.bitmap, res, TEST_BASE, arch::TABLE_SIZE, 1);
    }

    #[test]
    fn test_alloc_one_entry_aligned_skip() {
        let mut owner = OwnedBitmap::new(4, TEST_BASE);
        let mut bitmap = owner.bitmap();
        let mut state_machine =
            state_machine(bitmap, arch::TABLE_SIZE, arch::TABLE_SIZE as usize * 64);

        let res = state_machine.do_step(0);

        assert_eq!(res, Ok(TEST_BASE));
        assert_eq!(state_machine.scan_state, ScanState::Scanning,);
        assert_eq!(state_machine.bitmap.data[0], 0b1);

        state_machine.scan_state = ScanState::Scanning; // Reset the state machine to scanning to test the skip strategy
        let res = state_machine.do_step(0);
        assert_eq!(res, Err(SkipStrategy::NextAligned));
        let res = state_machine.do_step(1).unwrap();

        validate_allocation(
            state_machine.bitmap,
            res,
            TEST_BASE + (arch::TABLE_SIZE as u64 * 64),
            arch::TABLE_SIZE,
            arch::TABLE_SIZE as usize * 64,
        );
    }

    #[test]
    fn test_alloc_entry_unaligned_gt_entry() {
        let mut owner = OwnedBitmap::new(4, TEST_BASE);
        let mut bitmap = owner.bitmap();
        let mut state_machine =
            state_machine(bitmap, arch::TABLE_SIZE * 65, arch::TABLE_SIZE as usize);

        let res = state_machine.do_step(0);

        assert_eq!(res, Err(SkipStrategy::Next));
        assert_eq!(
            state_machine.scan_state,
            ScanState::Allocating {
                start: BitPtr::new(0, 0),
                remaining_size: arch::TABLE_SIZE
            }
        );

        // Step through the next entry, which should satisfy the allocation request
        let res = state_machine.do_step(1);

        validate_allocation(
            state_machine.bitmap,
            res.expect("should allocate"),
            TEST_BASE,
            arch::TABLE_SIZE * 65,
            arch::TABLE_SIZE as usize,
        );
    }

    #[test]
    fn test_alloc_entry_aligned_lt_entry() {
        let mut owner = OwnedBitmap::new(4, TEST_BASE);
        let mut bitmap = owner.bitmap();
        bitmap[0] = range_mask(9);
        let mut state_machine =
            state_machine(bitmap, arch::TABLE_SIZE / 2, arch::TABLE_SIZE as usize * 4);

        let res = state_machine.do_step(0);

        validate_allocation(
            state_machine.bitmap,
            res.expect("should allocate"),
            TEST_BASE + (12 * arch::TABLE_SIZE as u64),
            arch::TABLE_SIZE / 2,
            arch::TABLE_SIZE as usize * 4,
        )
    }
}
