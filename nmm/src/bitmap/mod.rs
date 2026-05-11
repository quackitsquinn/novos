use core::mem::Alignment;
use core::ops::BitOr;

use core::ops::{Index, IndexMut};
use core::simd::{num::SimdUint, u64x4};

mod allocate;
mod bitptr;
mod managers;
pub use bitptr::BitPtr;
pub use managers::virt::VirtualAddressManager;

use crate::test_println;

/// A bitmap primitive for tracking the allocation status of pages in the memory manager.
pub struct Bitmap<'a> {
    /// The base address of the memory region that this bitmap is managing, in bytes. This isn't guaranteed to be an address and is up to the user to interpret.
    base_addr: u64,
    /// The number of bits in the bitmap, which corresponds to the number of pages it can manage.
    n_bits: u64,
    /// A pointer to the bitmap data, which is a slice of u64 values where each bit represents the allocation status of a page.
    data: &'a mut [u64],
}

impl<'a> core::fmt::Debug for Bitmap<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Bitmap")
            .field("base_addr", &format_args!("{:#x}", self.base_addr))
            .field("n_bits", &self.n_bits)
            .finish()
    }
}

impl<'a> Bitmap<'a> {
    /// Initializes a bitmap with the given data slice, number of bits, and base address. The data slice must be large enough to hold at least `n_bits` bits (i.e., it must have a length of at least `n_bits / 64`).
    ///
    pub fn init(data: &'a mut [u64], n_bits: u64, base_addr: u64) -> Self {
        #[cfg(debug_assertions)]
        {
            assert!(
                n_bits <= data.len() as u64 * 64,
                "Number of bits exceeds capacity of data slice ({} bits available, but {} bits requested)",
                data.len() as u64 * 64,
                n_bits
            );
            if n_bits < (data.len() as f64 * 64.0 * 0.75) as u64 {
                use cake::log::warn;
                warn!(
                    "Bitmap is only using {}% of the capacity of the data slice. Consider reducing the size of the data slice to save memory.",
                    (n_bits as f64 / (data.len() as f64 * 64.0)) * 100.0
                );
            }
            assert!(
                data.len() as u64 * 64 >= n_bits,
                "Data slice is too small to hold the specified number of bits"
            );
        }
        Bitmap {
            base_addr,
            n_bits,
            data,
        }
    }

    /// Returns a u64x4 SIMD vector containing 4 consecutive entries from the bitmap data, starting at the specified entry index.
    ///
    /// This is useful for SIMD operations on the bitmap data.
    pub(crate) fn get_vec(&self, entry: usize) -> u64x4 {
        let start = entry;
        let end = start + 4;
        let slice = &self.data[start..end];
        u64x4::from_slice(slice)
    }

    /// Writes a u64x4 SIMD vector containing 4 consecutive entries to the bitmap data, starting at the specified entry index.
    pub(crate) fn set_vec(&mut self, entry: usize, vec: u64x4) {
        let start = entry;
        let end = start + 4;
        let slice = &mut self.data[start..end];
        vec.copy_to_slice(slice);
    }

    /// Returns an iterator over the bitmap data in chunks of 4 entries, yielding each chunk as a u64x4 SIMD vector.
    pub(crate) fn vec_chunks(&self) -> impl Iterator<Item = u64x4> {
        self.data
            .chunks_exact(4)
            .map(|chunk| u64x4::from_slice(chunk))
    }

    /// Returns an iterator over the bitmap data in chunks of `align` bits. This will iterate over every entry if align < 64.
    ///
    /// Returns (index of chunk, chunk) pairs.
    pub(crate) fn aligned_entries(&self, align: Alignment) -> impl Iterator<Item = (usize, u64)> {
        let align = align.as_usize();
        let ent_skip = (align / 64).max(1);
        self.data
            .chunks(ent_skip)
            .enumerate()
            .map(move |(i, chunk)| (i * ent_skip, chunk[0]))
    }

    /// Finds the first clear bit in the bitmap and returns a `BitPtr` pointing to it. If all bits are set, returns `None`.
    pub fn first_clear(&self) -> Option<BitPtr> {
        for (base, entry) in self.vec_chunks().enumerate() {
            let trailing_ones = entry.trailing_ones();
            if trailing_ones == u64x4::splat(64) {
                continue;
            }

            let res = trailing_ones
                .as_array()
                .iter()
                .enumerate()
                .find_map(|(i, entry)| {
                    if *entry == 64 {
                        return None;
                    }
                    Some(BitPtr::new((base * 4 + i) as u64, *entry as u8))
                });

            if let Some(bit_ptr) = res {
                return Some(bit_ptr);
            }
        }

        None
    }

    /// Sets or clears a range of bits in the bitmap, starting from the bit pointed to by `bit_ptr` and spanning `count` bits. If `value` is `true`, the bits will be set (marked as allocated); if `value` is `false`, the bits will be cleared (marked as free).
    pub fn set(&mut self, bit_ptr: BitPtr, count: u64) {
        self.mod_range(bit_ptr, count, u64::bitor, u64x4::bitor);
    }

    /// Clears a range of bits in the bitmap, starting from the bit pointed to by `bit_ptr` and spanning `count` bits. This marks the corresponding pages as free.
    pub fn clear(&mut self, bit_ptr: BitPtr, count: u64) {
        self.mod_range(bit_ptr, count, |a, b| a & !b, |a, b| a & !b);
    }

    /// Resets the entire bitmap, clearing all bits and marking all pages as free.
    pub fn reset(&mut self) {
        self.data.fill(0);
    }

    fn mod_range<Norm, Simd>(
        &mut self,
        bit_ptr: BitPtr,
        count: u64,
        mask_op: Norm,
        simd_mask_op: Simd,
    ) where
        Norm: Fn(u64, u64) -> u64,
        Simd: Fn(u64x4, u64x4) -> u64x4,
    {
        if bit_ptr.bit_index() + count > self.n_bits {
            panic!("Bit range exceeds bitmap bounds");
        }
        let start_bit = bit_ptr.bit_index();
        let end_bit = start_bit + count;
        assert!(end_bit <= self.n_bits, "Bit range exceeds bitmap bounds");
        let entry_index = bit_ptr.entry_index() as usize;
        let bit_offset = bit_ptr.bit_offset();

        if count < 64 || (bit_offset == 0 && count == 64) || (bit_offset == 0 && count <= 128) {
            match entry_mask(bit_offset, count) {
                Ok(mask) => self.data[entry_index] = mask_op(self.data[entry_index], mask),
                Err((mask, remaining)) => {
                    self.data[entry_index] = mask_op(self.data[entry_index], mask);
                    self.data[entry_index + 1] =
                        mask_op(self.data[entry_index + 1], bit_run_mask(remaining));
                }
            }
            return;
        }

        self.simd_mod(entry_index, bit_offset, count, mask_op, simd_mask_op);
    }

    fn simd_mod<Norm, Simd>(
        &mut self,
        start_entry: usize,
        bit_offset: u8,
        count: u64,
        mask_op: Norm,
        simd_mask_op: Simd,
    ) where
        Norm: Fn(u64, u64) -> u64,
        Simd: Fn(u64x4, u64x4) -> u64x4,
    {
        let (start_mask, remaining) = entry_mask(bit_offset, count).unwrap_err();
        let needed_entries = remaining.div_ceil(64);
        let end_mask = {
            let modulo = remaining % 64;
            if modulo == 0 {
                u64::MAX
            } else {
                bit_run_mask(modulo)
            }
        };
        let end_entry = start_entry + needed_entries as usize;

        self.data[start_entry] = mask_op(self.data[start_entry], start_mask);
        self.data[end_entry] = mask_op(self.data[end_entry], end_mask);

        let full_start = start_entry + 1;
        let full_end = end_entry - 1;

        let mut i = full_start;
        let mut rest: Option<(usize, usize)> = None;
        let mask = u64x4::splat(u64::MAX);
        while i <= full_end {
            let next = i + 4;

            if next > full_end {
                rest = Some((i, full_end));
                break;
            }

            let src = self.get_vec(i);
            let res = simd_mask_op(src, mask);
            #[cfg(test)]
            {
                println!("simd: writing to entries {} to {} with mask", i, i + 3,);
                for lane in 0..4 {
                    println!("simd:   mask_lane[{lane}] {:064b}", mask[lane]);
                }
                for lane in 0..4 {
                    println!("simd:   src_lane[{lane}]: {:064b}", src[lane]);
                }
            }
            self.set_vec(i, res);

            i = next;
        }

        if rest.is_none() {
            return;
        }

        let (mut i, full_end) = rest.unwrap();

        while i <= full_end {
            test_println!("Fallback at entry {}", i);
            self.data[i] = mask_op(self.data[i], u64::MAX);
            i += 1;
        }
    }
}

impl<'a> Index<usize> for Bitmap<'a> {
    type Output = u64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<'a> IndexMut<usize> for Bitmap<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

/// Creates a bitmask for a run of `count` bits starting from the least significant bit. For example, if `count` is 5, the returned bitmask will be `0b11111`.
///
/// This will saturate at `u64::MAX` if `count` is 64 or greater, since a u64 can only hold 64 bits. If `count` is 0, the returned bitmask will be 0.
pub const fn bit_run_mask(count: u64) -> u64 {
    if count == 0 {
        0
    } else if count >= 64 {
        u64::MAX
    } else {
        (1u64 << count) - 1
    }
}

/// Creates a bitmask for a range of bits starting at `bit_offset` and spanning `size` bits.
///
/// # Returns
///
/// `Ok(u64)` - A bitmask with the specified range of bits set to 1.
///
/// `Err((mask, remaining bits))` - A bitmask with the specified range of bits set to 1, but the range exceeds the bounds of a u64 (i.e., it tries to set bits beyond the 63rd bit).
/// The returned bitmask will have all bits from `bit_offset` to the end of the u64 set to 1 in this case. The `remaining bits` value indicates how many bits were out of bounds and could
/// not be set in the returned bitmask.
pub const fn entry_mask(bit_offset: u8, size: u64) -> Result<u64, (u64, u64)> {
    assert!(bit_offset < 64, "bit_offset must be less than 64");
    if size == 0 {
        return Ok(0);
    }
    if bit_offset as u64 + size > 64 {
        let valid_bits = 64 - bit_offset as u64;
        let mask = bit_run_mask(valid_bits).unbounded_shl(bit_offset as u32);
        return Err((mask, size - valid_bits));
    }
    Ok(bit_run_mask(size).unbounded_shl(bit_offset as u32))
}

#[cfg(test)]
mod tests {
    use std::simd::u64x4;

    use crate::bitmap::BitPtr;

    /// assert_eq but it prints the binary representation
    // implementation is mostly a copy and paste of assert_eq!, sans the internal panic message formatting.
    macro_rules! bit_assert_eq {
        ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                   panic!(
                        "assertion failed: `(left == right)`\n  left: `{:064b}`,\n right: `{:064b}`",
                        *left_val,
                        *right_val
                    );
                }
            }
        }};

    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    panic!(
                        "assertion failed: `(left == right)`\n  left: `{:064b}`,\n right: `{:064b}`: {}",
                        *left_val,
                        *right_val,
                        format_args!($($arg)+)
                    );
                }
            }
        }
    };

    }

    #[test]
    fn test_bit_run_mask() {
        bit_assert_eq!(super::bit_run_mask(0), 0);
        bit_assert_eq!(super::bit_run_mask(1), 1);
        bit_assert_eq!(super::bit_run_mask(5), 0b11111);
        bit_assert_eq!(super::bit_run_mask(64), u64::MAX);
        bit_assert_eq!(super::bit_run_mask(65), u64::MAX);
    }

    #[test]
    fn test_entry_mask() {
        assert_eq!(super::entry_mask(0, 0), Ok(0));
        assert_eq!(super::entry_mask(0, 1), Ok(1));
        assert_eq!(super::entry_mask(0, 64), Ok(u64::MAX));
        assert_eq!(super::entry_mask(1, 3), Ok(0b1110));
        assert_eq!(super::entry_mask(60, 5), Err((0b11111 << 60, 1)));
        assert_eq!(super::entry_mask(0, 128), Err((u64::MAX, 64)));
    }

    #[test]
    fn test_get_vec() {
        let mut data = [0u64; 64];
        data.iter_mut().enumerate().for_each(|(i, x)| *x = i as u64);

        let bitmap = unsafe { super::Bitmap::init(&mut data, 64 * 64, 0) };

        let vec = bitmap.get_vec(0);
        assert_eq!(vec, u64x4::from_array([0, 1, 2, 3]));

        let vec = bitmap.get_vec(1);
        assert_eq!(vec, u64x4::from_array([1, 2, 3, 4]));
    }

    #[test]
    fn test_set_vec() {
        let mut data = [0u64; 64];
        let mut bitmap = unsafe { super::Bitmap::init(&mut data, 64 * 64, 0) };

        let val = u64x4::splat(u64::MAX);
        bitmap.set_vec(0, val);
        assert_eq!(bitmap.get_vec(0), val);
    }

    #[test]
    fn test_vec_chunks() {
        let mut data = [0u64; 64];
        data.iter_mut().enumerate().for_each(|(i, x)| *x = i as u64);

        let bitmap = unsafe { super::Bitmap::init(&mut data, 64 * 64, 0) };

        let mut chunks = bitmap.vec_chunks();

        for i in (0..64).step_by(4) {
            let chunk = chunks.next().unwrap();
            assert_eq!(
                chunk,
                u64x4::from_array([i as u64, (i + 1) as u64, (i + 2) as u64, (i + 3) as u64])
            );
        }
    }

    #[test]
    fn test_first_clear() {
        let mut data = [0u64; 64];
        let bitmap = unsafe { super::Bitmap::init(&mut data, 64 * 64, 0) };

        assert_eq!(bitmap.first_clear(), Some(BitPtr::new(0, 0)));

        bitmap.data[0] = u64::MAX;
        assert_eq!(bitmap.first_clear(), Some(BitPtr::new(1, 0)));

        for entry in bitmap.data.iter_mut() {
            *entry = u64::MAX;
        }
        assert_eq!(bitmap.first_clear(), None);
    }

    #[test]
    fn test_set() {
        let mut data = [0u64; 64];
        let mut bitmap = unsafe { super::Bitmap::init(&mut data, 64 * 64, 0) };

        macro_rules! case {
            ($entry_index:expr, $bit_offset:expr,  $count:expr, $blk:block) => {
                bitmap.set(BitPtr::new($entry_index as u64, $bit_offset as u8), $count);
                $blk
                bitmap.data.fill(0);
            };
        }

        case!(0, 0, 1, {
            bit_assert_eq!(bitmap.data[0], 1);
            assert!(bitmap.data[1..].iter().all(|&x| x == 0));
        });

        case!(0, 0, 64, {
            bit_assert_eq!(bitmap.data[0], u64::MAX);
            assert!(bitmap.data[1..].iter().all(|&x| x == 0));
        });

        case!(0, 1, 3, {
            bit_assert_eq!(bitmap.data[0], 0b1110);
            assert!(bitmap.data[1..].iter().all(|&x| x == 0));
        });

        case!(0, 60, 5, {
            bit_assert_eq!(bitmap.data[0], 0b1111 << 60);
            bit_assert_eq!(bitmap.data[1], 0b1);
            assert!(bitmap.data[2..].iter().all(|&x| x == 0));
        });

        case!(1, 0, 128, {
            bit_assert_eq!(bitmap.data[1], u64::MAX);
            bit_assert_eq!(bitmap.data[2], u64::MAX);
            assert!(bitmap.data[3..].iter().all(|&x| x == 0));
        });

        case!(2, 32, 64, {
            bit_assert_eq!(bitmap.data[2], 0xFFFF_FFFF_0000_0000);
            bit_assert_eq!(bitmap.data[3], 0x0000_0000_FFFF_FFFF);
            assert!(bitmap.data[4..].iter().all(|&x| x == 0));
        });

        case!(0, 0, 64 * 64, {
            for i in 0..64 {
                bit_assert_eq!(bitmap.data[i], u64::MAX, "Entry {} should be fully set", i);
            }
        });

        case!(0, 32, 64 * 32, {
            bit_assert_eq!(bitmap.data[0], 0xFFFF_FFFF_0000_0000);
            for i in 1..31 {
                bit_assert_eq!(bitmap.data[i], u64::MAX, "Entry {} should be fully set", i);
            }
            bit_assert_eq!(bitmap.data[32], 0x0000_0000_FFFF_FFFF);
            assert!(bitmap.data[33..].iter().all(|&x| x == 0));
        });
    }

    #[test]
    fn test_clear() {
        let mut data = [u64::MAX; 64];
        let mut bitmap = unsafe { super::Bitmap::init(&mut data, 64 * 64, 0) };

        // Since 99% of the functionality of clear is shared with set, we don't need to retest all the same cases.
        // We just want to make a few sanity checks to make sure the mask operation is correctly clearing bits instead of setting them.
        macro_rules! case {
            ($entry_index:expr, $bit_offset:expr,  $count:expr, $blk:block) => {
                bitmap.clear(BitPtr::new($entry_index as u64, $bit_offset as u8), $count);
                $blk
                bitmap.data.fill(u64::MAX);
            };
        }

        case!(0, 1, 3, {
            bit_assert_eq!(bitmap.data[0], !0b1110);
            assert!(bitmap.data[1..].iter().all(|&x| x == u64::MAX));
        });

        case!(0, 0, 64, {
            bit_assert_eq!(bitmap.data[0], 0);
            assert!(bitmap.data[1..].iter().all(|&x| x == u64::MAX));
        });

        case!(0, 60, 5, {
            bit_assert_eq!(bitmap.data[0], !(0b1111 << 60));
            bit_assert_eq!(bitmap.data[1], !0b1);
            assert!(bitmap.data[2..].iter().all(|&x| x == u64::MAX));
        });

        case!(0, 0, 32 * 64, {
            for i in 0..32 {
                bit_assert_eq!(bitmap.data[i], 0);
            }
            assert!(bitmap.data[32..].iter().all(|&x| x == u64::MAX));
        });
    }
}
