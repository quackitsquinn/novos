//! A trait for accessing page tables at different levels in the page table hierarchy.

use crate::{
    MemError, arch,
    paging::{Address, AddressExt, PageTable, PageTableIndex, VirtAddr},
};

/// A trait for accessing page tables at different levels in the page table hierarchy.
pub trait PagetableAccessor {
    /// Reads the level 4 page table for the current architecture.
    fn read_l4_table(&self) -> Result<VirtAddr, MemError>;
    /// Reads the level 3 page table for the given index of the higher-level page table.
    fn read_l3_table(&self, l4_index: PageTableIndex) -> Result<VirtAddr, MemError>;
    /// Reads the level 2 page table for the given indices of the higher-level page tables.
    fn read_l2_table(
        &self,
        l4_index: PageTableIndex,
        l3_index: PageTableIndex,
    ) -> Result<VirtAddr, MemError>;
    /// Reads the level 1 page table for the given indices of the higher-level page tables.
    fn read_l1_table(
        &self,
        l4_index: PageTableIndex,
        l3_index: PageTableIndex,
        l2_index: PageTableIndex,
    ) -> Result<VirtAddr, MemError>;

    /// Reads the page table specified by the given `TablePointer`.
    fn read_table(&self, table_ptr: &TablePointer) -> Result<VirtAddr, MemError> {
        match table_ptr.level() {
            3 => self.read_l3_table(dissolve_address(table_ptr.address()).0),
            2 => {
                let (l4, l3, _, _) = dissolve_address(table_ptr.address());
                self.read_l2_table(l4, l3)
            }
            1 => {
                let (l4, l3, l2, _) = dissolve_address(table_ptr.address());
                self.read_l1_table(l4, l3, l2)
            }
            _ => unreachable!("Invalid table level: {}", table_ptr.level()),
        }
    }

    /// Returns a reference to the level 4 page table.
    fn l4_table(&self) -> Result<&PageTable, MemError> {
        let addr = self.read_l4_table()?;
        let table: &PageTable = unsafe { &*(addr.as_ptr()) };
        Ok(table)
    }

    /// Returns a reference to the level 3 page table for the given index of the higher-level page table.
    fn l3_table(&self, l4_index: PageTableIndex) -> Result<&PageTable, MemError> {
        let addr = self.read_l3_table(l4_index)?;
        let table: &PageTable = unsafe { &*(addr.as_ptr()) };
        Ok(table)
    }

    /// Returns a reference to the level 2 page table for the given indices of the higher-level page tables.
    fn l2_table(
        &self,
        l4_index: PageTableIndex,
        l3_index: PageTableIndex,
    ) -> Result<&PageTable, MemError> {
        let addr = self.read_l2_table(l4_index, l3_index)?;
        let table: &PageTable = unsafe { &*(addr.as_ptr()) };
        Ok(table)
    }

    /// Returns a reference to the level 1 page table for the given indices of the higher-level page tables.
    fn l1_table(
        &self,
        l4_index: PageTableIndex,
        l3_index: PageTableIndex,
        l2_index: PageTableIndex,
    ) -> Result<&PageTable, MemError> {
        let addr = self.read_l1_table(l4_index, l3_index, l2_index)?;
        let table: &PageTable = unsafe { &*(addr.as_ptr()) };
        Ok(table)
    }
}

/// A pointer to a page table at a specific level in the page table hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TablePointer {
    table: VirtAddr,
}

impl TablePointer {
    fn new(
        l4_idx: PageTableIndex,
        l3_idx: PageTableIndex,
        l2_idx: PageTableIndex,
        level: u8,
    ) -> Self {
        let table = VirtAddr::new(
            build_address(l4_idx, l3_idx, l2_idx, PageTableIndex::new(0)) | level as u64,
        );
        Self { table }
    }

    /// Creates a new `TablePointer` for a level 4 page table, given the index for the level 4 page table.
    pub fn new_l3(l4_idx: PageTableIndex) -> Self {
        Self::new(l4_idx, PageTableIndex::new(0), PageTableIndex::new(0), 3)
    }

    /// Creates a new `TablePointer` for a level 2 page table, given the indices for the level 4 and level 3 page tables.
    pub fn new_l2(l4_idx: PageTableIndex, l3_idx: PageTableIndex) -> Self {
        Self::new(l4_idx, l3_idx, PageTableIndex::new(0), 2)
    }

    /// Creates a new `TablePointer` for a level 1 page table, given the indices for the level 4, level 3, and level 2 page tables.
    pub fn new_l1(l4_idx: PageTableIndex, l3_idx: PageTableIndex, l2_idx: PageTableIndex) -> Self {
        Self::new(l4_idx, l3_idx, l2_idx, 1)
    }

    /// Returns the level of the page table this pointer points to (1, 2, 3, or 4).
    pub fn level(&self) -> u8 {
        (self.table.as_u64() & 0x3) as u8
    }

    /// Returns the virtual address pointing up to the page table at the specified level.
    pub fn address(&self) -> VirtAddr {
        VirtAddr::new(self.table.as_u64() & !0x3)
    }

    /// Dissolves the virtual address of the page table into its constituent indices for each level of the page table hierarchy.
    pub fn dissolve(&self) -> (PageTableIndex, PageTableIndex, PageTableIndex) {
        let (l4_idx, l3_idx, l2_idx, _) = dissolve_address(self.address());
        (l4_idx, l3_idx, l2_idx)
    }
}

/// Builds a virtual address from the given page table indices for each level of the page table hierarchy.
pub fn build_address(
    l4_idx: PageTableIndex,
    l3_idx: PageTableIndex,
    l2_idx: PageTableIndex,
    l1_idx: PageTableIndex,
) -> u64 {
    let mut addr: u64 = 0;
    addr |= (l4_idx.value() as u64) << (arch::TABLE_INDEX_BITS * 3 + arch::ENTRY_OFFSET_BITS);
    addr |= (l3_idx.value() as u64) << (arch::TABLE_INDEX_BITS * 2 + arch::ENTRY_OFFSET_BITS);
    addr |= (l2_idx.value() as u64) << (arch::TABLE_INDEX_BITS + arch::ENTRY_OFFSET_BITS);
    addr |= (l1_idx.value() as u64) << arch::ENTRY_OFFSET_BITS;
    addr
}

/// Dissolves a virtual address into its constituent page table indices for each level of the page table hierarchy.
///
/// This does not account for larger sized pages.
pub fn dissolve_address(
    addr: VirtAddr,
) -> (
    PageTableIndex,
    PageTableIndex,
    PageTableIndex,
    PageTableIndex,
) {
    let addr_val = addr.as_u64();
    let l4_idx = PageTableIndex::new(
        ((addr_val >> (arch::TABLE_INDEX_BITS * 3 + arch::ENTRY_OFFSET_BITS))
            & ((1 << arch::TABLE_INDEX_BITS) - 1)) as u16,
    );
    let l3_idx = PageTableIndex::new(
        ((addr_val >> (arch::TABLE_INDEX_BITS * 2 + arch::ENTRY_OFFSET_BITS))
            & ((1 << arch::TABLE_INDEX_BITS) - 1)) as u16,
    );
    let l2_idx = PageTableIndex::new(
        ((addr_val >> (arch::TABLE_INDEX_BITS + arch::ENTRY_OFFSET_BITS))
            & ((1 << arch::TABLE_INDEX_BITS) - 1)) as u16,
    );
    let l1_idx = PageTableIndex::new(
        ((addr_val >> arch::ENTRY_OFFSET_BITS) & ((1 << arch::TABLE_INDEX_BITS) - 1)) as u16,
    );
    (l4_idx, l3_idx, l2_idx, l1_idx)
}
