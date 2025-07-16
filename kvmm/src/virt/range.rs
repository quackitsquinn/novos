use x86_64::{
    VirtAddr,
    structures::paging::{Page, page::PageRangeInclusive},
};

use crate::{KernelPage, KernelPageSize};

/// A range of virtual addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualAddressRange {
    /// The start of the range.
    pub start: VirtAddr,
    /// The size of the range.
    pub size: u64,
}

impl VirtualAddressRange {
    /// Create a new virtual address range with the given start and size.
    pub fn new(start: VirtAddr, size: u64) -> Self {
        Self { start, size }
    }

    /// Create a new virtual address range with the given start and size, ensuring that the address is page aligned.
    pub fn new_aligned(start: VirtAddr, size: u64) -> Self {
        assert!(
            start.as_u64() % 4096 == 0,
            "{:?} is not page aligned",
            start
        );
        assert!(size % 4096 == 0, "{:#x} is not size aligned", size);
        Self { start, size }
    }

    /// Create a new virtual address range with the given start and size, ensuring that the address is page aligned and the size is a multiple of the page size.
    pub fn new_page(start: VirtAddr) -> Self {
        Self::new_aligned(start, 4096)
    }

    /// Create a new virtual address range with the given start and size, ensuring that the address is page aligned and the size is a multiple of the page size.
    /// The size is in pages.
    pub fn new_page_range(start: VirtAddr, size: u64) -> Self {
        Self::new_aligned(start, size * 4096)
    }

    /// Check if the given address and size are contained within this range.
    pub fn contains(&self, addr: VirtAddr, size: u64) -> bool {
        addr >= self.start && addr < self.start + self.size && addr + size <= self.start + self.size
    }

    /// Check if the given range is contained within this range.
    pub fn contains_range(&self, range: &VirtualAddressRange) -> bool {
        self.contains(range.start, range.size)
    }

    /// Takes n bytes out of the range, returning a new range with the taken bytes.
    pub fn take(&mut self, size: u64) -> Option<VirtualAddressRange> {
        if size > self.size {
            return None;
        }

        let range = VirtualAddressRange::new(self.start + size, self.size - size);
        self.size -= size;
        Some(range)
    }

    /// Returns the end of the range.
    #[inline(always)]
    pub fn end(&self) -> VirtAddr {
        self.start + self.size
    }

    /// Extends the range by the given size.
    /// Returns the new end of the range.
    pub fn extend(&mut self, size: u64) {
        self.size += size;
    }

    /// Returns the range as a page range.
    pub fn as_page_range(&self) -> Option<PageRangeInclusive<KernelPageSize>> {
        if self.size == 0 || self.size % 4096 != 0 || self.start.as_u64() % 4096 != 0 {
            return None;
        }
        let start = Page::containing_address(self.start);
        let end = Page::containing_address(self.end() - 1u64);
        Some(Page::range_inclusive(start, end))
    }

    /// Returns the range as a page.
    pub fn as_page(&self) -> KernelPage {
        assert!(self.size == 4096, "Range is not a single page");
        assert!(self.start.as_u64() % 4096 == 0, "Range is not page aligned");
        Page::from_start_address(self.start).expect("infallible")
    }

    /// Returns the range as a pointer.
    pub fn as_ptr<T>(&self) -> *mut T {
        self.start.as_mut_ptr()
    }
}

#[cfg(test)]
mod tests {
    use x86_64::structures::paging::PageSize;

    use super::*;

    fn new(start: u64, size: u64) -> VirtualAddressRange {
        VirtualAddressRange::new(VirtAddr::new(start), size)
    }

    #[test]
    fn test_new() {
        let range = new(0x1000, 0x2000);
        assert_eq!(range.start, VirtAddr::new(0x1000));
        assert_eq!(range.size, 0x2000);
    }

    #[test]
    fn test_new_aligned() {
        let range = VirtualAddressRange::new_aligned(VirtAddr::new(0x1000), 0x2000);
        assert_eq!(range.start, VirtAddr::new(0x1000));
        assert_eq!(range.size, 0x2000);
    }

    #[test]
    #[should_panic]
    fn test_new_aligned_not_page_aligned() {
        VirtualAddressRange::new_aligned(VirtAddr::new(0x1001), 0x2000);
    }

    #[test]
    fn test_new_page_range() {
        let range = VirtualAddressRange::new_page_range(VirtAddr::new(0x1000), 4);
        assert_eq!(range.start, VirtAddr::new(0x1000));
        assert_eq!(range.size, 0x4000);
    }

    #[test]
    fn test_contains() {
        let range = new(0x1000, 0x2000);
        assert!(range.contains(VirtAddr::new(0x1000), 0x1000));
        assert!(range.contains(VirtAddr::new(0x1800), 0x200));
        assert!(!range.contains(VirtAddr::new(0x3000), 0x1000));
        assert!(!range.contains(VirtAddr::new(0x1000), 0x3000));
    }

    #[test]
    fn test_contains_range() {
        let range = new(0x1000, 0x2000);
        let cases = [
            (new(0x1000, 0x1000), true),
            (new(0x1800, 0x200), true),
            (new(0x3000, 0x1000), false),
            (new(0x1000, 0x3000), false),
        ];
        for (sub_range, expected) in cases {
            assert_eq!(
                range.contains_range(&sub_range),
                expected,
                "Failed for sub-range: {:?}",
                sub_range
            );
        }
    }

    #[test]
    fn test_take_none() {
        let mut range = new(0x1000, 0x2000);
        assert_eq!(range.take(0x3000), None);
    }

    #[test]
    fn test_take() {
        let mut range = new(0x1000, 0x2000);
        let taken = range.take(0x1000).unwrap();
        assert_eq!(range.start, VirtAddr::new(0x1000));
        assert_eq!(range.size, 0x1000);
        assert_eq!(taken.start, VirtAddr::new(0x2000));
        assert_eq!(taken.size, 0x1000);
    }

    #[test]
    fn test_end() {
        let range = new(0x1000, 0x2000);
        assert_eq!(range.end(), VirtAddr::new(0x3000));
    }

    #[test]
    fn test_extend() {
        let mut range = new(0x1000, 0x2000);
        range.extend(0x1000);
        assert_eq!(range.size, 0x3000);
        assert_eq!(range.end(), VirtAddr::new(0x4000));
    }

    #[test]
    fn test_as_page_range() {
        let range = new(0x1000, 0x2000);
        let page_range = range.as_page_range().expect("Failed to get page range");
        assert_eq!(
            page_range.start,
            Page::containing_address(VirtAddr::new(0x1000))
        );
        assert_eq!(
            page_range.end,
            Page::containing_address(VirtAddr::new(0x2FFF))
        );
    }

    #[test]
    fn test_as_page_range_invalid() {
        let cases = [
            new(0x1000, 0x1FFF), // Not page aligned size
            new(0x1001, 0x2000), // Not page aligned start
            new(0x1000, 0),      // Zero size
        ];
        for range in cases {
            assert!(
                range.as_page_range().is_none(),
                "Expected None for range: {:?}",
                range
            );
        }
    }

    #[test]
    fn test_as_page() {
        let range = new(0x1000, 0x1000);
        let page = range.as_page();
        assert_eq!(page.start_address(), VirtAddr::new(0x1000));
        assert_eq!(page.size(), KernelPageSize::SIZE);
    }

    #[test]
    fn test_as_ptr() {
        let range = new(0x1000, 0x1000);
        let ptr: *mut u8 = range.as_ptr();
        assert_eq!(ptr, range.start.as_mut_ptr());
    }
}
