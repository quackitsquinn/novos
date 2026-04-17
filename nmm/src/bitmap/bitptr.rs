/// A pointer to a specific bit in a bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitPtr(u64);

impl BitPtr {
    /// Creates a new `BitPtr` from a given bit index.
    pub fn new(entry_index: u64, bit_offset: u8) -> Self {
        assert!(bit_offset < 64, "bit_offset must be less than 64");
        assert!(
            entry_index < (u64::MAX >> 8),
            "entry_index is too large to fit in a BitPtr"
        );
        BitPtr((entry_index << 8) | (bit_offset as u64))
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
}
