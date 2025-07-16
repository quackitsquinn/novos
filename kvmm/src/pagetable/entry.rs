use cake::Owned;
use x86_64::{
    VirtAddr,
    structures::paging::{PageTable, PageTableIndex},
};

use crate::pagetable::PageTablePath;

/// Represents a single pagetable in a PageLayout.
pub struct Entry {
    pagetable: Owned<PageTable>,
    index: PageIndex,
}

impl Entry {
    /// Create a new entry for the given pagetable and path.
    pub fn new(pagetable: Owned<PageTable>, v_addr: PageTablePath) -> Entry {
        Entry {
            pagetable: pagetable,
            index: PageIndex::pack(v_addr),
        }
    }

    /// Get the pagetable associated with this entry.
    pub fn pagetable(&mut self) -> &mut PageTable {
        &mut self.pagetable
    }

    /// Get the pagetable path associated with this entry.
    pub fn path(&self) -> PageTablePath {
        self.index.unpack()
    }

    /// Get the raw path of this entry as a PageIndex.
    /// This is a packed representation of the page table indexes.
    pub fn raw_path(&self) -> PageIndex {
        self.index
    }
}

/// A packed representation of a page table index.
/// It contains the P4, P3, and P2 indexes packed into a single u64 value
/// with the last two bits indicating the presence of P3 and P2 indexes.
/// - 0: Only P4 index is present.
/// - 1: Only P4 and P3 indexes are present.
/// - 2: P4, P3, and P2 indexes are present.
/// This allows for efficient storage and retrieval of page table paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageIndex(u64);

impl PageIndex {
    /// Packs the given indexes into a single u64 value.
    pub fn pack(indexes: PageTablePath) -> Self {
        let (p4_index, p3_index, p2_index) = indexes;
        let vaddr = indexes_to_virtaddr((
            p4_index,
            p3_index.unwrap_or(PageTableIndex::new(0)),
            p2_index.unwrap_or(PageTableIndex::new(0)),
        ));
        let c = if p2_index.is_some() {
            2
        } else if p3_index.is_some() {
            1
        } else {
            0
        };

        let value = vaddr.as_u64() | c as u64;
        PageIndex(value)
    }

    /// Unpacks the u64 value into the original indexes.
    pub fn unpack(
        &self,
    ) -> (
        PageTableIndex,
        Option<PageTableIndex>,
        Option<PageTableIndex>,
    ) {
        let virt_addr = VirtAddr::new(self.0);
        let n = self.0 & 0b11;
        let p4_index = virt_addr.p4_index();

        let p3_index = if n >= 1 {
            Some(virt_addr.p3_index())
        } else {
            None
        };

        let p2_index = if n == 2 {
            Some(virt_addr.p2_index())
        } else {
            None
        };

        (p4_index, p3_index, p2_index)
    }
}

#[inline(always)]
fn indexes_to_virtaddr(indexes: (PageTableIndex, PageTableIndex, PageTableIndex)) -> VirtAddr {
    let (p4_index, p3_index, p2_index) = indexes;
    let mut addr = 0;
    addr |= u64::from(p4_index) << 39;
    addr |= u64::from(p3_index) << 30;
    addr |= u64::from(p2_index) << 21;
    VirtAddr::new(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_index_new() {
        let paths = [
            (PageTableIndex::new(1), Some(PageTableIndex::new(2)), None),
            (
                PageTableIndex::new(1),
                Some(PageTableIndex::new(2)),
                Some(PageTableIndex::new(3)),
            ),
            (PageTableIndex::new(1), None, None),
        ];
        for path in paths {
            let page_index = PageIndex::pack(path);
            assert_eq!(page_index.unpack(), path);
        }
    }
}
