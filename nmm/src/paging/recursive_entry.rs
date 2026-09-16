use crate::paging::PageTableIndex;

pub struct RecursiveEntryManager {
    base: PageTableIndex,
    indices: u16,
    n_entries: u8,
}

impl RecursiveEntryManager {
    pub const unsafe fn new(base: PageTableIndex, n_entries: u8) -> Self {
        Self {
            base,
            indices: 0,
            n_entries,
        }
    }

    const fn read_idx(&self, idx: u8) -> bool {
        if idx >= self.n_entries {
            panic!("Index out of bounds");
        }
        (self.indices & (1 << idx)) != 0
    }

    const fn set_idx(&mut self, idx: u8, value: bool) {
        if idx >= self.n_entries {
            panic!("Index out of bounds");
        }
        if value {
            self.indices |= 1 << idx;
        } else {
            self.indices &= !(1 << idx);
        }
    }

    /// Reserves an index for use, returning the reserved index if successful, or None if no indices are available.
    pub const fn reserve(&mut self) -> Option<PageTableIndex> {
        let idx = self.indices.trailing_ones() as u8;
        if idx >= self.n_entries {
            None
        } else {
            self.set_idx(idx, true);
            Some(PageTableIndex::new(self.base.value() + idx as u16))
        }
    }

    /// Releases a previously reserved index, making it available for future reservations.
    ///
    /// # Safety
    /// The given index must have been previously reserved by this manager. Releasing an index that was not reserved may lead to undefined behavior.
    pub const unsafe fn release(&mut self, idx: PageTableIndex) {
        let idx = idx.value() - self.base.value();
        if idx >= self.n_entries as u16 {
            panic!("Index out of bounds");
        }
        self.set_idx(idx as u8, false);
    }
}

impl Default for RecursiveEntryManager {
    fn default() -> Self {
        let range = crate::arch::USEABLE_RECURSIVE_SLOTS;
        let base = range.start;
        let n_entries = u8::min((range.end.value() - base.value()).min(255) as u8, 16);
        Self {
            base,
            indices: 0,
            n_entries: n_entries,
        }
    }
}
