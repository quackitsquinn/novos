use core::fmt::Debug;

/// A pointer to a specific bit in a bitmap.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct BitPtr(u64);

impl BitPtr {
    /// A `BitPtr` that points to the first bit in the bitmap (entry index 0, bit offset 0).
    pub const ZERO: Self = BitPtr(0);

    /// Creates a new `BitPtr` from a given bitmap entry. The `entry` is shifted left by 8 bits to make room for the bit offset, which is set to 0.
    pub fn entry(entry: u64) -> Self {
        BitPtr(entry << 8)
    }

    /// Creates a new `BitPtr` from a given bit index.
    pub fn new(entry_index: u64, bit_offset: u8) -> Self {
        assert!(bit_offset < 64, "bit_offset must be less than 64");
        assert!(
            entry_index < (u64::MAX >> 8),
            "entry_index is too large to fit in a BitPtr"
        );
        BitPtr((entry_index << 8) | (bit_offset as u64))
    }

    /// Creates a new `BitPtr` from a given bitmap entry and bit offset, wrapping the bit_offset and adding it to the entry index.
    ///
    /// `entry_index` will still panic if it overflows
    pub fn new_wrapping(entry_index: u64, bit_offset: u64) -> Self {
        let entries_from_bits = bit_offset / 64;
        let bit_offset = (bit_offset % 64) as u8;
        let entry_index = entry_index + entries_from_bits;
        Self::new(entry_index, bit_offset)
    }

    /// Returns the index of the bitmap entry that this `BitPtr` points to.
    pub fn entry_index(&self) -> u64 {
        self.0 >> 8
    }

    /// Returns the offset of the bit within the bitmap entry that this `BitPtr` points to.
    pub fn bit_offset(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// Returns the overall bit index that this `BitPtr` points to, calculated as `entry_index * 64 + bit_offset`.
    pub fn bit_index(&self) -> u64 {
        self.entry_index() * 64 + self.bit_offset() as u64
    }

    /// Returns a pointer to the bit in the bitmap that this `BitPtr` points to,
    /// given the base address of the range and the size of the underlying memory each bit represents in bytes.
    pub fn as_ptr<T>(&self, base: *const u8, bit_size: u64) -> *const T {
        let byte_offset = self.bit_index() * bit_size;
        let res = (base as u64).checked_add(byte_offset);
        match res {
            Some(val) => val as *const T,
            None => panic!("BitPtr offset overflow"),
        }
    }

    /// Returns a mutable pointer to the bit in the bitmap that this `BitPtr` points to,
    /// given the base address of the range and the size of the underlying memory each bit represents in bytes.
    pub fn as_mut_ptr<T>(&self, base: *mut u8, bit_size: u64) -> *mut T {
        let byte_offset = self.bit_index() * bit_size;
        let res = (base as u64).checked_add(byte_offset);
        match res {
            Some(val) => val as *mut T,
            None => panic!("BitPtr offset overflow"),
        }
    }

    /// Checks if adding `n_bits` to the current bit offset would overflow into the next bitmap entry.
    pub fn will_overflow(&self, n_bits: u64) -> bool {
        let bit_index = self.bit_offset() as u64 + n_bits;
        return bit_index > 64;
    }
}

impl Debug for BitPtr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BitPtr({}:{})", self.entry_index(), self.bit_offset())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_bitptr() {
        let bit_ptr = super::BitPtr::new(5, 10);
        assert_eq!(bit_ptr.entry_index(), 5);
        assert_eq!(bit_ptr.bit_offset(), 10);
        assert_eq!(bit_ptr.bit_index(), 5 * 64 + 10);

        let bit_ptr = super::BitPtr::new_wrapping(5, 130);
        assert_eq!(bit_ptr.entry_index(), 7);
        assert_eq!(bit_ptr.bit_offset(), 2);
        assert_eq!(bit_ptr.bit_index(), 7 * 64 + 2);
    }

    #[test]
    fn test_bitptr_overflow() {
        let bit_ptr = super::BitPtr::new(5, 10);
        assert!(!bit_ptr.will_overflow(50));
        assert!(bit_ptr.will_overflow(55));
    }
}
