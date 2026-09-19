use arrayvec::ArrayVec;

use crate::{
    MapFlags, MemError,
    paging::{
        Address, FragmentManager, Frame, Large, Medium, MemoryFragment, PageTable, PageTableEntry,
        PageTableIndex, Small, VirtAddr,
        accessor::{self, PagetableAccessor},
        map::{Flush, MemoryMapper, SizedMemoryMapper, Unmapped},
        primitives::AnyPage,
        table,
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

    fn l3_table_or_alloc(
        &mut self,
        l4_index: PageTableIndex,
        table_flags: MapFlags,
        allocator: &mut impl FragmentManager<Frame<Small>, Small>,
    ) -> Result<&mut PageTable, MemError> {
        let l4_entry = self.p4_mut().read_entry(l4_index);
        if !l4_entry.is_present() {
            let new_frame = allocator.allocate_fragment()?;
            unsafe {
                self.p4_mut()
                    .set_entry(l4_index, PageTableEntry::new(new_frame, table_flags));
            }
            let table = self.l3_table_mut(l4_index)?;
            unsafe { table.zero() };
            return Ok(table);
        }
        self.l3_table_mut(l4_index)
    }

    fn l2_table_or_alloc(
        &mut self,
        l4_index: PageTableIndex,
        l3_index: PageTableIndex,
        table_flags: MapFlags,
        allocator: &mut impl FragmentManager<Frame<Small>, Small>,
    ) -> Result<&mut PageTable, MemError> {
        let l3_table = self.l3_table_or_alloc(l4_index, table_flags, allocator)?;
        let l3_entry = l3_table.read_entry(l3_index);
        if !l3_entry.is_present() {
            let new_frame = allocator.allocate_fragment()?;
            unsafe {
                l3_table.set_entry(l3_index, PageTableEntry::new(new_frame, table_flags));
            }
            let table = self.l2_table_mut(l4_index, l3_index)?;
            unsafe { table.zero() };
            return Ok(table);
        }
        self.l2_table_mut(l4_index, l3_index)
    }

    fn l1_table_or_alloc(
        &mut self,
        l4_index: PageTableIndex,
        l3_index: PageTableIndex,
        l2_index: PageTableIndex,
        table_flags: MapFlags,
        allocator: &mut impl FragmentManager<Frame<Small>, Small>,
    ) -> Result<&mut PageTable, MemError> {
        let l2_table = self.l2_table_or_alloc(l4_index, l3_index, table_flags, allocator)?;
        let l2_entry = l2_table.read_entry(l2_index);
        if !l2_entry.is_present() {
            let new_frame = allocator.allocate_fragment()?;
            unsafe {
                l2_table.set_entry(l2_index, PageTableEntry::new(new_frame, table_flags));
            }
            let table = self.l1_table_mut(l4_index, l3_index, l2_index)?;
            unsafe { table.zero() };
            return Ok(table);
        }
        self.l1_table_mut(l4_index, l3_index, l2_index)
    }
}

impl SizedMemoryMapper<Large> for RecursivePageTable<'_> {
    fn map_primitive<A>(
        &mut self,
        dst: super::Page<Large>,
        src: super::Frame<Large>,
        flags: crate::MapFlags,
        parent_table_flags: Option<crate::MapFlags>,
        allocator: &mut A,
    ) -> Result<super::map::Flush, crate::MemError>
    where
        A: super::FragmentManager<super::Frame<super::Small>, super::Small>,
    {
        let (l4, l3, _, _) = accessor::dissolve_address(dst.start_address());

        let table_flags = parent_table_flags.unwrap_or_default() | MapFlags::WRITABLE;
        let l3_table = self.l3_table_or_alloc(l4, table_flags, allocator)?;
        unsafe {
            l3_table.set_entry(l3, PageTableEntry::new(src, flags));
        }
        Ok(unsafe { Flush::flush_page(dst) })
    }

    unsafe fn unmap_primitive(
        &mut self,
        page: super::Page<Large>,
    ) -> Result<Unmapped<Large>, crate::MemError> {
        let (l4, l3, _, _) = accessor::dissolve_address(page.start_address());
        let table = self.l3_table_mut(l4)?;
        let entry = table.read_entry(l3);
        if !entry.is_present() {
            return Err(MemError::NotMapped(AnyPage::Large(page)));
        } else if !entry.is_huge() {
            return Err(MemError::MappedToLowerLevel(page.start_address()));
        }
        unsafe {
            table.set_entry(l3, PageTableEntry::empty());
        }

        Ok(Unmapped::new(
            Frame::from_start_address(entry.addr()).unwrap(),
            accessor::find_free_parents_for(page, &mut *self)?,
            unsafe { Some(Flush::flush_page(page)) },
            entry.flags(),
        ))
    }
}

impl SizedMemoryMapper<Medium> for RecursivePageTable<'_> {
    fn map_primitive<A>(
        &mut self,
        dst: super::Page<Medium>,
        src: super::Frame<Medium>,
        flags: crate::MapFlags,
        parent_table_flags: Option<crate::MapFlags>,
        allocator: &mut A,
    ) -> Result<super::map::Flush, crate::MemError>
    where
        A: super::FragmentManager<super::Frame<super::Small>, super::Small>,
    {
        let (l4, l3, l2, _) = accessor::dissolve_address(dst.start_address());

        let table_flags = parent_table_flags.unwrap_or_default() | MapFlags::WRITABLE;
        let l2_table = self.l2_table_or_alloc(l4, l3, table_flags, allocator)?;
        unsafe {
            l2_table.set_entry(l2, PageTableEntry::new(src, flags));
        }
        Ok(unsafe { Flush::flush_page(dst) })
    }

    unsafe fn unmap_primitive(
        &mut self,
        page: super::Page<Medium>,
    ) -> Result<super::map::Unmapped<Medium>, crate::MemError> {
        let (l4, l3, l2, _) = accessor::dissolve_address(page.start_address());
        let table = self.l2_table_mut(l4, l3)?;
        let entry = table.read_entry(l2);
        if !entry.is_present() {
            return Err(MemError::NotMapped(AnyPage::Medium(page)));
        } else if !entry.is_huge() {
            return Err(MemError::MappedToLowerLevel(page.start_address()));
        }
        unsafe {
            table.set_entry(l2, PageTableEntry::empty());
        }

        Ok(Unmapped::new(
            Frame::from_start_address(entry.addr()).unwrap(),
            accessor::find_free_parents_for(page, &mut *self)?,
            unsafe { Some(Flush::flush_page(page)) },
            entry.flags(),
        ))
    }
}

impl SizedMemoryMapper<Small> for RecursivePageTable<'_> {
    fn map_primitive<A>(
        &mut self,
        dst: super::Page<Small>,
        src: super::Frame<Small>,
        flags: crate::MapFlags,
        parent_table_flags: Option<crate::MapFlags>,
        allocator: &mut A,
    ) -> Result<super::map::Flush, crate::MemError>
    where
        A: super::FragmentManager<super::Frame<super::Small>, super::Small>,
    {
        let (l4, l3, l2, l1) = accessor::dissolve_address(dst.start_address());

        let table_flags = parent_table_flags.unwrap_or_default() | MapFlags::WRITABLE;
        let l1_table = self.l1_table_or_alloc(l4, l3, l2, table_flags, allocator)?;
        unsafe {
            l1_table.set_entry(l1, PageTableEntry::new(src, flags));
        }
        Ok(unsafe { Flush::flush_page(dst) })
    }

    unsafe fn unmap_primitive(
        &mut self,
        page: super::Page<Small>,
    ) -> Result<super::map::Unmapped<Small>, crate::MemError> {
        let (l4, l3, l2, l1) = accessor::dissolve_address(page.start_address());
        let table = self.l1_table_mut(l4, l3, l2)?;
        let entry = table.read_entry(l1);
        if !entry.is_present() {
            return Err(MemError::NotMapped(AnyPage::Small(page)));
        } else if !entry.is_huge() {
            return Err(MemError::MappedToLowerLevel(page.start_address()));
        }
        unsafe {
            table.set_entry(l1, PageTableEntry::empty());
        }

        Ok(Unmapped::new(
            Frame::from_start_address(entry.addr()).unwrap(),
            accessor::find_free_parents_for(page, &mut *self)?,
            unsafe { Some(Flush::flush_page(page)) },
            entry.flags(),
        ))
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

        Ok(VirtAddr::new_truncate(addr_raw))
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
            return Err(MemError::MappedToHigherLevel(VirtAddr::new_truncate(
                accessor::build_address(
                    l4_index,
                    l3_index,
                    PageTableIndex::new(0),
                    PageTableIndex::new(0),
                ),
            )));
        }

        Ok(VirtAddr::new_truncate(addr_raw))
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
            return Err(MemError::MappedToHigherLevel(VirtAddr::new_truncate(
                accessor::build_address(l4_index, l3_index, l2_index, PageTableIndex::new(0)),
            )));
        }

        Ok(VirtAddr::new_truncate(addr_raw))
    }
}

impl MemoryMapper for RecursivePageTable<'_> {}
