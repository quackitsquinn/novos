//! Page index type.

/// A index into a page table. This value will always be less than the current platform's page table entry count (512 for x86_64).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    /// Creates a new `PageTableIndex` from a raw value. The caller must ensure that the value is valid (i.e., less than the entry count for the current architecture).
    pub const unsafe fn new_unchecked(value: u16) -> Self {
        Self(value)
    }

    /// Creates a new `PageTableIndex` from a raw value, returning `None` if the value is out of bounds.
    pub const fn try_new(value: u16) -> Option<Self> {
        if value < crate::arch::ENTRY_COUNT as u16 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Creates a new `PageTableIndex` from a raw value, panicking if the value is out of bounds.
    pub const fn new(value: u16) -> Self {
        Self::try_new(value).expect("PageTableIndex value out of bounds")
    }

    /// Returns the raw index value.
    pub const fn value(self) -> u16 {
        self.0
    }
}
