use x86_64::VirtAddr;

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

        let range = VirtualAddressRange::new(self.start, size);
        self.start += size;
        self.size -= size;
        Some(range)
    }

    /// Returns the end of the range.
    #[inline(always)]
    pub fn end(&self) -> VirtAddr {
        self.start + self.size
    }
}
