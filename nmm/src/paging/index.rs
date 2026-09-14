//! Page index type.

/// A index into a page table. This value will always be less than the current platform's page table entry count (512 for x86_64).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    /// The minimum valid page table index (0).
    pub const MIN: PageTableIndex = PageTableIndex(0);
    /// The maximum valid page table index (ENTRY_COUNT - 1).
    pub const MAX: PageTableIndex = PageTableIndex(crate::arch::ENTRY_COUNT as u16 - 1);

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
    #[inline(always)]
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Returns the raw index value as a `u64`.
    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }

    /// Returns an iterator over the range of `PageTableIndex` values from `start` to `end`.
    pub const fn iter_range(range: core::ops::Range<PageTableIndex>) -> PageIndexIter {
        PageIndexIter {
            current: range.start,
            end: range.end,
        }
    }

    /// Returns an iterator over all possible `PageTableIndex` values for the current architecture.
    pub const fn iter_all() -> PageIndexIter {
        Self::iter_range(PageTableIndex(0)..PageTableIndex(crate::arch::ENTRY_COUNT as u16))
    }
}

pub struct PageIndexIter {
    current: PageTableIndex,
    end: PageTableIndex,
}

impl Iterator for PageIndexIter {
    type Item = PageTableIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            let idx = self.current;
            self.current = PageTableIndex(self.current.value() + 1);
            Some(idx)
        } else {
            None
        }
    }
}
