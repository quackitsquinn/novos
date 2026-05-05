use core::simd::u64x4;
use core::{mem::Alignment, simd};

use crate::align;

use crate::bitmap::{BitPtr, Bitmap, bitptr};
use crate::test_println;

impl<'a> Bitmap<'a> {
    /// Checks if any of the `n_bits` bits starting from the bit pointed to by `bit_ptr` are set in the bitmap. Returns `true` if any of the bits are set, and `false` otherwise.
    pub fn bits_are_set(&self, bit_ptr: BitPtr, n_bits: u64) -> bool {
        let first_mask = super::bit_run_mask(n_bits) << bit_ptr.bit_offset();
        let entry_idx = bit_ptr.entry_index() as usize;
        let first_entry = self[entry_idx];
        if first_entry & first_mask != 0 {
            return true;
        }

        if !bit_ptr.will_overflow(n_bits) {
            // Simplest case, just mask `first_entry` and check if it's zero
            let res = first_entry & first_mask;
            return res != 0;
        }

        let full_entries = (bit_ptr.bit_offset() as u64 + n_bits) / 64;
        let overflowed_entries = full_entries % 4;
        let entry_idx = entry_idx + 1;

        // Check so that we can guarantee SIMD alignment
        for i in 0..overflowed_entries {
            let entry = self[entry_idx + i as usize];
            if entry != 0 {
                return true;
            }
        }

        let simd_start = entry_idx + overflowed_entries as usize;
        let simd_end = simd_start + full_entries as usize;

        for i in (simd_start..simd_end).step_by(4) {
            let entries = self.get_vec(i);
            if entries != u64x4::splat(0) {
                return true;
            }
        }

        let unaligned_bits = (bit_ptr.bit_offset() as u64 + n_bits) % 64;
        let last_mask = super::bit_run_mask(n_bits) >> unaligned_bits;
        let last_entry_idx = bit_ptr.entry_index() + full_entries;
        let last_entry = self[last_entry_idx as usize];
        if last_entry & last_mask != 0 {
            return true;
        }

        return false;
    }

    /// Allocates a contiguous run of `n_bits` clear bits in the bitmap, aligned to `align`.
    /// This is the slow path for when `n_bits` is greater than 64, as we have to check multiple entries for each potential allocation.
    ///
    /// Note that this should still be reasonably fast for large allocations, due to several optimizations you can make with large allocations:
    /// - If the alignment is greater than 64, we can skip entire entries that are not zero, as they will not be able to fit the allocation.
    /// - `self.bits_are_set` is optimized for large runs of bits, using SIMD to check multiple entries at a time. Operations that are aligned to at least 4 entries (256 bits) will run only through SIMD code,
    ///    which should be faster than unaligned checks.
    /// - `self.set` is also optimized for large runs of bits, using SIMD to set multiple entries at a time.
    ///
    /// Returns a `BitPtr` pointing to the first allocated bit, or `None` if no suitable run of clear bits is found.
    fn allocate_large(&mut self, n_bits: u64, align: Alignment) -> Option<BitPtr> {
        let per_entry_align = align.as_usize() > 64;
        let mut res = None;
        for (ind, entry) in self.aligned_entries(align) {
            // guaranteed to be aligned, so if the entry is not 0, we can skip it entirely
            if per_entry_align {
                if entry != 0 {
                    continue;
                }
                let bitptr = BitPtr::entry(ind as u64);
                if self.bits_are_set(bitptr, n_bits) {
                    continue;
                }
                res = Some(bitptr);
                break;
            }

            // we check the leading zeros because they will be directly next to the trailing zeros in the next entry,
            // as all alignments here will be greater than 64.
            let end_free = entry.leading_zeros() as u64;
            let offset = 64 - end_free;
            let rounded = align!(up, offset, align.as_usize() as u64);
            if rounded >= 64 {
                continue;
            }

            let bitptr = BitPtr::new(ind as u64, rounded as u8);
            if self.bits_are_set(bitptr, n_bits) {
                continue;
            }
            res = Some(bitptr);
            break;
        }

        let res = res?;
        self.set(res, n_bits);
        Some(res)
    }

    /// Allocates a contiguous run of `n_bits` clear bits in the bitmap, aligned to `align`. Returns a `BitPtr` pointing to the first allocated bit, or `None` if no suitable run of clear bits is found.
    pub fn allocate(&mut self, n_bits: u64, align: Alignment) -> Option<BitPtr> {
        if n_bits > 64 {
            return self.allocate_large(n_bits, align);
        }
        let n_bits = n_bits as u32;
        let mut iter = self.aligned_entries(align);
        let mut res = None;
        'outer: for (ind, entry) in iter.by_ref() {
            test_println!("entry: {:064b}", entry);
            if entry == u64::MAX {
                continue;
            }

            let mut curr = !entry;
            let mut base = 0;
            while curr != 0 {
                test_println!("curr: {:064b}", curr);
                let tz = curr.trailing_zeros();
                curr >>= tz;

                let offset = base + tz;

                let size = curr.trailing_ones();
                let aligned_offset = align!(up, offset, align.as_usize() as u32);
                test_println!("tz: {}, size: {}, align_base: {}", tz, size, aligned_offset);
                let effective_size = size.saturating_sub(aligned_offset - offset);

                test_println!(
                    "tz: {}, size: {}, align_base: {}, effective_size: {}",
                    tz,
                    size,
                    aligned_offset,
                    effective_size
                );

                if effective_size >= n_bits as u32 {
                    test_println!("found a run at entry {}, bit {}", ind, aligned_offset);
                    let bitptr = BitPtr::new(ind as u64, aligned_offset as u8);
                    debug_assert!(!self.bits_are_set(bitptr, n_bits as u64));
                    res = Some(bitptr);
                    break 'outer;
                }
                curr = curr.unbounded_shr(size);
                base += tz + size;
            }
        }
        let res = res?;
        drop(iter);
        self.set(res, n_bits as u64);
        return Some(res);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bitmap,
        paging::{Large, Medium, PrimitiveSize},
    };

    use super::*;

    #[test]
    fn test_run_mask() {
        macro_rules! case {
            ($size: expr, $align: expr, $expected_mask: expr, $expected_n_reps: expr) => {
                assert_eq!(
                    run_mask($size, Alignment::new($align).unwrap()).unwrap(),
                    ($expected_n_reps, $expected_mask),
                    "run_mask({}, {}) is not equal to {:064b}",
                    $size,
                    $align,
                    run_mask($size, Alignment::new($align).unwrap()).unwrap().1,
                );
            };

            ($size: expr, $align: expr) => {
                assert!(
                    run_mask($size, Alignment::new($align).unwrap()).is_none(),
                    "run_mask({}, {}) should be None",
                    $size,
                    $align
                );
            };

            // capture more than 2 arguments
            ($size: expr, $align: expr; $($rest:tt)*) => {
                case!($size, $align);
                case!($($rest)*);
            };

            ($size: expr, $align: expr, $expected_mask: expr, $expected_n_reps: expr; $($rest:tt)*) => {
                case!($size, $align, $expected_mask, $expected_n_reps);
                case!($($rest)*);
            };
        }

        case!(
            3, 8, 0b0000_0111_0000_0111_0000_0111_0000_0111_0000_0111_0000_0111_0000_0111_0000_0111, 8;
            8,8, u64::MAX, 8;
            8, 32, 0b0000_0000_0000_0000_0000_0000_1111_1111_0000_0000_0000_0000_0000_0000_1111_1111, 2;
            32, 128, 0x0000_0000_FFFF_FFFF, 1;
            33, 32;
            2, 1;
            128, 128
        );
    }

    #[test]
    fn test_bits_are_set() {
        let mut data = [0u64; 64];
        let mut bitmap = unsafe { super::Bitmap::init(&mut data, 64 * 64, 0) };

        let check_set = |bitmap: &super::Bitmap, bit_ptr: BitPtr, n_bits: u64, expected: bool| {
            assert_eq!(
                bitmap.bits_are_set(bit_ptr, n_bits),
                expected,
                "bits_are_set({:?}, {}) should be {}",
                bit_ptr,
                n_bits,
                expected
            );
        };

        bitmap.set(BitPtr::ZERO, 512);
        for i in 0..512 {
            check_set(&bitmap, BitPtr::new_wrapping(0, i), 512, true);
        }

        for i in 512..1025 {
            check_set(&bitmap, BitPtr::new_wrapping(0, i), 512, false);
        }
        bitmap.reset();

        bitmap.set(BitPtr::new(1, 0), 64);
        check_set(&bitmap, BitPtr::new(0, 0), 512, true);
        bitmap.reset();

        bitmap.set(BitPtr::new(0, 2), 2);
        check_set(&bitmap, BitPtr::new(0, 1), 512, true);
        check_set(&bitmap, BitPtr::new(0, 1), 1, false);
        check_set(&bitmap, BitPtr::new(0, 1), 2, true);
        check_set(&bitmap, BitPtr::new(0, 0), 1, false);
    }

    #[test]
    fn test_allocate() {
        const CAP: usize = 0x200000 * 2;
        // move this massive array onto the heap to avoid a stack overflow in debug mode
        let mut data = vec![0; CAP];
        let mut bitmap = unsafe { super::Bitmap::init(&mut data, 64 * CAP as u64, 0) };

        let alloc = |bitmap: &mut super::Bitmap, n_bits: u64, align: usize| {
            let res = bitmap.allocate(
                n_bits,
                Alignment::new(align).expect("provided align not power of two!"),
            );
            assert!(
                res.is_some(),
                "allocate({}, {}) should return Some(BitPtr), got None",
                n_bits,
                align
            );
            res.unwrap()
        };

        assert_eq!(alloc(&mut bitmap, 1, 1), BitPtr::new(0, 0));
        assert_eq!(alloc(&mut bitmap, 1, 1), BitPtr::new(0, 1));
        assert_eq!(alloc(&mut bitmap, 2, 2), BitPtr::new(0, 2));
        assert_eq!(alloc(&mut bitmap, 3, 8), BitPtr::new(0, 8));
        assert_eq!(alloc(&mut bitmap, 8, 8), BitPtr::new(0, 16));
        assert_eq!(alloc(&mut bitmap, 8, 32), BitPtr::new(0, 32));
        assert_eq!(alloc(&mut bitmap, 128, 128), BitPtr::new(2, 0));
        assert_eq!(
            alloc(&mut bitmap, Medium::SIZE, Medium::SIZE as usize),
            BitPtr::new(Medium::SIZE / 64, 0)
        );
        assert_eq!(alloc(&mut bitmap, 34, 512), BitPtr::new(512 / 64, 0));
    }
}
