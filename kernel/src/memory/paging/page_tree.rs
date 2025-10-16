use x86_64::{
    VirtAddr,
    structures::paging::{
        OffsetPageTable, Page, PageTable, PageTableFlags, PageTableIndex, RecursivePageTable,
        page_table::PageTableEntry,
    },
};

use crate::memory::paging::builder::{PageTableBuilder, RECURSIVE_ENTRY_INDEX};

/// This trait provides an interface for accessing different levels of the page table hierarchy.
/// It allows for retrieving immutable and mutable references to page tables at various levels.
pub trait PageTree {
    fn get_pml4(&self) -> &PageTable;

    fn get_pml4_mut(&mut self) -> &mut PageTable;

    fn get_l3(&self, pml4_index: PageTableIndex) -> Option<&PageTable>;

    unsafe fn get_l3_mut(&mut self, pml4_index: PageTableIndex) -> Option<&mut PageTable>;

    fn get_l2(&self, pml4_index: PageTableIndex, pdpt_index: PageTableIndex) -> Option<&PageTable>;

    unsafe fn get_l2_mut(
        &mut self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
    ) -> Option<&mut PageTable>;

    fn get_l1(
        &self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
        pd_index: PageTableIndex,
    ) -> Option<&PageTable>;

    unsafe fn get_l1_mut(
        &mut self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
        pd_index: PageTableIndex,
    ) -> Option<&mut PageTable>;

    /// Returns an immutable reference to a page table at the given path.
    fn get(&self, path: [Option<PageTableIndex>; 3]) -> Option<&PageTable> {
        //.Assert that [None, 0, 0] or [None, None, 0] are not allowed
        assert!(
            path.iter().skip_while(|p| p.is_none()).all(Option::is_none),
            "Invalid path"
        );
        match path {
            [None, None, None] => Some(self.get_pml4()),
            [Some(pml4), None, None] => self.get_l3(pml4),
            [Some(pml4), Some(pdpt), None] => self.get_l2(pml4, pdpt),
            [Some(pml4), Some(pdpt), Some(pd)] => self.get_l1(pml4, pdpt, pd),
            _ => None,
        }
    }

    /// Returns a mutable reference to a page table at the given path.
    unsafe fn get_mut(&mut self, path: [Option<PageTableIndex>; 3]) -> Option<&mut PageTable> {
        //.Assert that [None, 0, 0] or [None, None, 0] are not allowed
        assert!(
            path.iter().skip_while(|p| p.is_none()).all(Option::is_none),
            "Invalid path"
        );
        unsafe {
            match path {
                [None, None, None] => Some(self.get_pml4_mut()),
                [Some(pml4), None, None] => self.get_l3_mut(pml4),
                [Some(pml4), Some(pdpt), None] => self.get_l2_mut(pml4, pdpt),
                [Some(pml4), Some(pdpt), Some(pd)] => self.get_l1_mut(pml4, pdpt, pd),
                _ => None,
            }
        }
    }
}

impl PageTree for OffsetPageTable<'_> {
    fn get_pml4(&self) -> &PageTable {
        self.level_4_table()
    }

    fn get_pml4_mut(&mut self) -> &mut PageTable {
        self.level_4_table_mut()
    }

    fn get_l3(&self, pml4_index: PageTableIndex) -> Option<&PageTable> {
        let pml4 = self.level_4_table();
        let pml4_entry = &pml4[pml4_index];
        if !pml4_entry
            .flags()
            .contains(x86_64::structures::paging::PageTableFlags::PRESENT)
        {
            return None;
        }
        let pdpt_table: &PageTable = unsafe {
            &*(pml4_entry
                .addr()
                .as_u64()
                .wrapping_add(self.phys_offset().as_u64()) as *const _)
        };
        Some(pdpt_table)
    }

    unsafe fn get_l3_mut(&mut self, pml4_index: PageTableIndex) -> Option<&mut PageTable> {
        let hhdm_offset = self.phys_offset();
        let pml4 = self.level_4_table_mut();
        let pml4_entry = &mut pml4[pml4_index];
        get_table_offset(pml4_entry, hhdm_offset).map(|addr| unsafe { &mut *(addr.as_mut_ptr()) })
    }

    fn get_l2(&self, pml4_index: PageTableIndex, pdpt_index: PageTableIndex) -> Option<&PageTable> {
        let pdpt = self.get_l3(pml4_index)?;
        get_table_offset(&pdpt[pdpt_index], self.phys_offset())
            .map(|addr| unsafe { &*(addr.as_ptr()) })
    }

    unsafe fn get_l2_mut(
        &mut self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
    ) -> Option<&mut PageTable> {
        let hhdm_offset = self.phys_offset();
        let pdpt = unsafe { self.get_l3_mut(pml4_index)? };
        get_table_offset(&pdpt[pdpt_index], hhdm_offset)
            .map(|addr| unsafe { &mut *(addr.as_mut_ptr()) })
    }

    fn get_l1(
        &self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
        pd_index: PageTableIndex,
    ) -> Option<&PageTable> {
        let pd = self.get_l2(pml4_index, pdpt_index)?;
        get_table_offset(&pd[pd_index], self.phys_offset()).map(|addr| unsafe { &*(addr.as_ptr()) })
    }

    unsafe fn get_l1_mut(
        &mut self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
        pd_index: PageTableIndex,
    ) -> Option<&mut PageTable> {
        let hhdm_offset = self.phys_offset();
        let pd = unsafe { self.get_l2_mut(pml4_index, pdpt_index)? };
        get_table_offset(&pd[pd_index], hhdm_offset)
            .map(|addr| unsafe { &mut *(addr.as_mut_ptr()) })
    }
}

// Returns a pointer to the given page table entry's mapped frame, adjusted by the HHDM offset
fn get_table_offset(pte: &PageTableEntry, hhdm_offset: VirtAddr) -> Option<VirtAddr> {
    if pte.is_unused() {
        return None;
    }

    Some(VirtAddr::new(
        pte.addr().as_u64().wrapping_add(hhdm_offset.as_u64()),
    ))
}

impl PageTree for RecursivePageTable<'_> {
    fn get_pml4(&self) -> &PageTable {
        self.level_4_table()
    }

    fn get_pml4_mut(&mut self) -> &mut PageTable {
        self.level_4_table_mut()
    }

    fn get_l3(&self, pml4_index: PageTableIndex) -> Option<&PageTable> {
        let pml4 = self.level_4_table();
        let pml4_entry = &pml4[pml4_index];
        if !pml4_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        Some(unsafe {
            get_pagetable_recursive(RECURSIVE_ENTRY_INDEX, RECURSIVE_ENTRY_INDEX, pml4_index)
        })
    }

    unsafe fn get_l3_mut(&mut self, pml4_index: PageTableIndex) -> Option<&mut PageTable> {
        let pml4 = self.level_4_table_mut();
        let pml4_entry = &mut pml4[pml4_index];
        if !pml4_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        Some(unsafe {
            get_pagetable_recursive_mut(RECURSIVE_ENTRY_INDEX, RECURSIVE_ENTRY_INDEX, pml4_index)
        })
    }

    fn get_l2(&self, pml4_index: PageTableIndex, pdpt_index: PageTableIndex) -> Option<&PageTable> {
        let pdpt = self.get_l3(pml4_index)?;
        let pdpt_entry = &pdpt[pdpt_index];
        if !pdpt_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        Some(unsafe { get_pagetable_recursive(RECURSIVE_ENTRY_INDEX, pml4_index, pdpt_index) })
    }

    unsafe fn get_l2_mut(
        &mut self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
    ) -> Option<&mut PageTable> {
        let pdpt = unsafe { self.get_l3_mut(pml4_index)? };
        let pdpt_entry = &pdpt[pdpt_index];
        if !pdpt_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        Some(unsafe { get_pagetable_recursive_mut(RECURSIVE_ENTRY_INDEX, pml4_index, pdpt_index) })
    }

    fn get_l1(
        &self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
        pd_index: PageTableIndex,
    ) -> Option<&PageTable> {
        let pd = self.get_l2(pml4_index, pdpt_index)?;
        let pd_entry = &pd[pd_index];
        if !pd_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        Some(unsafe { get_pagetable_recursive(pml4_index, pdpt_index, pd_index) })
    }

    unsafe fn get_l1_mut(
        &mut self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
        pd_index: PageTableIndex,
    ) -> Option<&mut PageTable> {
        let pd = self.get_l2(pml4_index, pdpt_index)?;
        let pd_entry = &pd[pd_index];
        if !pd_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        Some(unsafe { get_pagetable_recursive_mut(pml4_index, pdpt_index, pd_index) })
    }
}

unsafe fn get_pagetable_recursive<'a>(
    pml4_index: PageTableIndex,
    pdpt_index: PageTableIndex,
    pd_index: PageTableIndex,
) -> &'a PageTable {
    let page =
        Page::from_page_table_indices(RECURSIVE_ENTRY_INDEX, pml4_index, pdpt_index, pd_index);
    let addr = page.start_address().as_u64();
    unsafe { &*(addr as *const PageTable) }
}

unsafe fn get_pagetable_recursive_mut<'a>(
    pml4_index: PageTableIndex,
    pdpt_index: PageTableIndex,
    pd_index: PageTableIndex,
) -> &'a mut PageTable {
    let page =
        Page::from_page_table_indices(RECURSIVE_ENTRY_INDEX, pml4_index, pdpt_index, pd_index);
    let addr = page.start_address().as_u64();
    unsafe { &mut *(addr as *mut PageTable) }
}
