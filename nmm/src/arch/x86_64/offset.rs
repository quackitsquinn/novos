use core::fmt::Debug;

use x86_64::structures::paging::{Mapper as _, Translate as _};

use crate::{
    MapFlags, MemError,
    arch::x86_64::{PageTableFlags, XFrameAllocator, impl_memory_mapper_for},
    paging::{
        Address, FragmentManager, Frame, Large, Medium, Page, PageTable, PageTableIndex, Small,
        VirtAddr,
        accessor::{self, PagetableAccessor},
        map::{Flush, MemoryMapper, SizedMemoryMapper, Unmapped},
    },
};

mod arch_lib {
    pub use x86_64::structures::paging::{OffsetPageTable, mapper::TranslateResult};
}

/// An offset page table mapper for x86_64. This mapper uses a fixed offset to access the page tables, and is the most basic type of mapper.
// We wrap x86_64's OffsetPageTable to prevent having x86_64 effect the internal implementation details of the mapper.

pub struct OffsetPageTable<'a> {
    inner: arch_lib::OffsetPageTable<'a>,
}

impl<'a> OffsetPageTable<'a> {
    /// Creates a new RecursivePageTable from a mutable reference to the level 4 page table and the recursive index.
    /// # Safety
    /// The caller must ensure that the provided page table is the actual level 4 page table, and that the recursive index is correctly set up in the page tables.
    pub unsafe fn new(table: &'a mut PageTable, offset: VirtAddr) -> Self {
        let table = unsafe { arch_lib::OffsetPageTable::new(table.as_arch_mut(), offset.into()) };
        Self { inner: table }
    }
    /// Returns the recursive index used for this recursive page table.
    pub fn phys_offset(&self) -> VirtAddr {
        self.inner.phys_offset().into()
    }

    /// Returns a reference to the level 4 page table.
    pub fn p4(&self) -> &PageTable {
        // Just as a programmer's note: I know p4 is x86_64 specific, but it's nicer and gets the point across quicker than level_4_table.
        // If another arch does commonly use more than 4 levels of page tables, then this function is named dumb. but that doesn't matter right now.
        PageTable::from_arch_ref(self.inner.level_4_table())
    }

    /// Returns a mutable reference to the level 4 page table.
    pub fn p4_mut(&mut self) -> &mut PageTable {
        PageTable::from_arch_mut(self.inner.level_4_table_mut())
    }
}

impl Debug for OffsetPageTable<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OffsetPageTable").finish()
    }
}

impl MemoryMapper for OffsetPageTable<'_> {}

impl_memory_mapper_for!(OffsetPageTable<'_>, Medium, true);
impl_memory_mapper_for!(OffsetPageTable<'_>, Large, true);
impl_memory_mapper_for!(OffsetPageTable<'_>, Small, false);

impl PagetableAccessor for OffsetPageTable<'_> {
    fn read_l4_table(&self) -> Result<VirtAddr, MemError> {
        Ok(self.p4().as_virt())
    }

    fn read_l3_table(&self, l4_index: crate::paging::PageTableIndex) -> Result<VirtAddr, MemError> {
        let l4 = self.p4();
        let entry = l4.read_entry(l4_index);
        if entry.is_present() {
            Ok(self.phys_offset() + entry.addr().as_u64())
        } else {
            Err(MemError::PagetableNotPresent)
        }
    }

    fn read_l2_table(
        &self,
        l4_index: crate::paging::PageTableIndex,
        l3_index: crate::paging::PageTableIndex,
    ) -> Result<VirtAddr, MemError> {
        let l3_table = self.l3_table(l4_index)?;
        let entry = l3_table.read_entry(l3_index);
        if entry.is_huge() {
            return Err(MemError::MappedToHigherLevel(
                self.phys_offset()
                    + accessor::build_address(
                        l4_index,
                        l3_index,
                        PageTableIndex::new(0),
                        PageTableIndex::new(0),
                    ),
            ));
        }
        if entry.is_present() {
            return Ok(self.phys_offset() + entry.addr().as_u64());
        }
        Err(MemError::PagetableNotPresent)
    }

    fn read_l1_table(
        &self,
        l4_index: crate::paging::PageTableIndex,
        l3_index: crate::paging::PageTableIndex,
        l2_index: crate::paging::PageTableIndex,
    ) -> Result<VirtAddr, MemError> {
        let l2_table = self.l2_table(l4_index, l3_index)?;
        let entry = l2_table.read_entry(l2_index);
        if entry.is_huge() {
            return Err(MemError::MappedToHigherLevel(
                self.phys_offset()
                    + accessor::build_address(l4_index, l3_index, l2_index, PageTableIndex::new(0)),
            ));
        }
        if entry.is_present() {
            return Ok(self.phys_offset() + entry.addr().as_u64());
        }
        Err(MemError::PagetableNotPresent)
    }
}
