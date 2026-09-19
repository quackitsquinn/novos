use crate::{
    MemError,
    paging::{
        Address, Large, Medium, PageTable, PageTableIndex, Small, VirtAddr,
        accessor::{self, PagetableAccessor},
        map::{MemoryMapper, SizedMemoryMapper},
    },
};

pub struct RecursivePageTable<'a> {
    pub(crate) mut(super) root_table: &'a mut PageTable,
    pub(crate) mut(super) recursive_index: PageTableIndex,
}

impl<'a> RecursivePageTable<'a> {
    pub(crate) unsafe fn new(
        root_table: &'a mut PageTable,
        recursive_index: PageTableIndex,
    ) -> Self {
        Self {
            root_table,
            recursive_index,
        }
    }

    pub(crate) fn p4(&self) -> &PageTable {
        self.root_table
    }

    pub(crate) fn p4_mut(&mut self) -> &mut PageTable {
        self.root_table
    }

    pub(crate) fn recursive_index(&self) -> PageTableIndex {
        self.recursive_index
    }
}

impl SizedMemoryMapper<Large> for RecursivePageTable<'_> {
    fn map_primitive<A>(
        &mut self,
        _dst: super::Page<Large>,
        _src: super::Frame<Large>,
        _flags: crate::MapFlags,
        _parent_table_flags: Option<crate::MapFlags>,
        _allocator: &mut A,
    ) -> Result<super::map::Flush, crate::MemError>
    where
        A: super::FragmentManager<super::Frame<super::Small>, super::Small>,
    {
        todo!()
    }

    unsafe fn unmap_primitive(
        &mut self,
        _page: super::Page<Large>,
    ) -> Result<super::map::Unmapped<Large>, crate::MemError> {
        todo!()
    }
}

impl SizedMemoryMapper<Medium> for RecursivePageTable<'_> {
    fn map_primitive<A>(
        &mut self,
        _dst: super::Page<Medium>,
        _src: super::Frame<Medium>,
        _flags: crate::MapFlags,
        _parent_table_flags: Option<crate::MapFlags>,
        _allocator: &mut A,
    ) -> Result<super::map::Flush, crate::MemError>
    where
        A: super::FragmentManager<super::Frame<super::Small>, super::Small>,
    {
        todo!()
    }

    unsafe fn unmap_primitive(
        &mut self,
        _page: super::Page<Medium>,
    ) -> Result<super::map::Unmapped<Medium>, crate::MemError> {
        todo!()
    }
}

impl SizedMemoryMapper<Small> for RecursivePageTable<'_> {
    fn map_primitive<A>(
        &mut self,
        _dst: super::Page<Small>,
        _src: super::Frame<Small>,
        _flags: crate::MapFlags,
        _parent_table_flags: Option<crate::MapFlags>,
        _allocator: &mut A,
    ) -> Result<super::map::Flush, crate::MemError>
    where
        A: super::FragmentManager<super::Frame<super::Small>, super::Small>,
    {
        todo!()
    }

    unsafe fn unmap_primitive(
        &mut self,
        _page: super::Page<Small>,
    ) -> Result<super::map::Unmapped<Small>, crate::MemError> {
        todo!()
    }
}

impl PagetableAccessor for RecursivePageTable<'_> {
    fn read_l4_table(&self) -> Result<crate::paging::VirtAddr, MemError> {
        Ok(self.p4().as_virt())
    }

    fn read_l3_table(&self, l4_index: PageTableIndex) -> Result<crate::paging::VirtAddr, MemError> {
        let addr_raw = accessor::build_address(
            self.recursive_index,
            self.recursive_index,
            self.recursive_index,
            l4_index,
        );

        if !self.p4().read_entry(l4_index).is_present() {
            return Err(MemError::PagetableNotPresent);
        }

        Ok(VirtAddr::new(addr_raw))
    }

    fn read_l2_table(
        &self,
        l4_index: PageTableIndex,
        l3_index: PageTableIndex,
    ) -> Result<crate::paging::VirtAddr, MemError> {
        let addr_raw = accessor::build_address(
            self.recursive_index,
            self.recursive_index,
            l4_index,
            l3_index,
        );

        let entry = self.l3_table(l4_index)?.read_entry(l3_index);

        if !entry.is_present() {
            return Err(MemError::PagetableNotPresent);
        }

        if entry.is_huge() {
            return Err(MemError::MappedToHigherLevel(VirtAddr::new(
                accessor::build_address(
                    l4_index,
                    l3_index,
                    PageTableIndex::new(0),
                    PageTableIndex::new(0),
                ),
            )));
        }

        Ok(VirtAddr::new(addr_raw))
    }

    fn read_l1_table(
        &self,
        l4_index: PageTableIndex,
        l3_index: PageTableIndex,
        l2_index: PageTableIndex,
    ) -> Result<crate::paging::VirtAddr, MemError> {
        let addr_raw = accessor::build_address(self.recursive_index, l4_index, l3_index, l2_index);

        let entry = self.l2_table(l4_index, l3_index)?.read_entry(l2_index);
        if !entry.is_present() {
            return Err(MemError::PagetableNotPresent);
        }

        if entry.is_huge() {
            return Err(MemError::MappedToHigherLevel(VirtAddr::new(
                accessor::build_address(l4_index, l3_index, l2_index, PageTableIndex::new(0)),
            )));
        }

        Ok(VirtAddr::new(addr_raw))
    }
}

impl MemoryMapper for RecursivePageTable<'_> {}
