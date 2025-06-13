use kserial::client::fs::File;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{page_table::PageTableEntry, Page, PageTable, PageTableIndex, PhysFrame},
    VirtAddr,
};

use crate::memory::paging::{virt, MEMORY_OFFSET};

#[derive(Debug, Clone, Copy)]
pub struct RecursivePageIterator {
    root_table: &'static PageTable,
    offset: u64,
    l1: Option<(PageTableIndex, &'static PageTable)>,
    l2: Option<(PageTableIndex, &'static PageTable)>,
    l3: Option<(PageTableIndex, &'static PageTable)>,
    l4: PageTableIndex,
}

impl RecursivePageIterator {
    pub fn new(root_table: &'static PageTable) -> Self {
        let offset = *MEMORY_OFFSET.get().expect("MEMORY_OFFSET not set");
        Self {
            root_table,
            offset,
            l1: None,
            l2: None,
            l3: None,
            l4: PageTableIndex::new(0),
        }
    }

    fn next_used(
        pagetable: &PageTable,
        offset: PageTableIndex,
    ) -> Option<(PageTableIndex, &PageTableEntry)> {
        pagetable
            .iter()
            .enumerate()
            .skip(offset.into())
            .find(|p| !p.1.is_unused())
            .map(|(index, pte)| (PageTableIndex::new(index as u16), pte))
    }

    fn next_l1(&mut self) -> Option<(PageTableIndex, &'static PageTable)> {
        let mut offset = PageTableIndex::new(0);
        if self.l1.is_some() {
            offset = self.l1.unwrap().0
        }

        let (index, pte) = Self::next_used(self.root_table, offset)?;

        let pagetable_addr = pte
            .frame()
            .expect("PageTableEntry should have a frame")
            .start_address()
            .as_u64()
            + self.offset;
        let pagetable = unsafe { &*(pagetable_addr as *const PageTable) };
        Some((index, pagetable))
    }

    fn next_l2(&mut self) -> Option<(PageTableIndex, &'static PageTable)> {
        let mut offset = PageTableIndex::new(0);
        if self.l2.is_some() {
            offset = self.l2.unwrap().0
        }

        if self.l1.is_none() {
            self.l1 = self.next_l1();
        }

        let next = Self::next_used(self.l1?.1, offset);

        if next.is_none() {
            self.l1 = self.next_l1();
            return self.next_l2();
        }

        let (index, pte) = next?;

        let pagetable_addr = pte
            .frame()
            .expect("PageTableEntry should have a frame")
            .start_address()
            .as_u64()
            + self.offset;
        let pagetable = unsafe { &*(pagetable_addr as *const PageTable) };
        Some((index, pagetable))
    }

    fn next_l3(&mut self) -> Option<(PageTableIndex, &'static PageTable)> {
        let mut offset = PageTableIndex::new(0);
        if self.l3.is_some() {
            offset = self.l3.unwrap().0
        }

        if self.l2.is_none() {
            self.l2 = self.next_l2();
        }

        let next = Self::next_used(self.l2?.1, offset);

        if next.is_none() {
            self.l2 = self.next_l2();
            return self.next_l3();
        }

        let (index, pte) = next?;

        let pagetable_addr = pte
            .frame()
            .expect("PageTableEntry should have a frame")
            .start_address()
            .as_u64()
            + self.offset;
        let pagetable = unsafe { &*(pagetable_addr as *const PageTable) };
        Some((index, pagetable))
    }

    fn next_l4(&mut self) -> Option<(PageTableIndex, &PageTableEntry)> {
        if self.l3.is_none() {
            self.l3 = self.next_l3();
        }

        let next = Self::next_used(self.l3?.1, self.l4);

        if next.is_none() {
            self.l3 = self.next_l3();
            return self.next_l4();
        }

        next
    }
}

impl Iterator for RecursivePageIterator {
    type Item = (Page, PhysFrame);

    fn next(&mut self) -> Option<Self::Item> {
        let (ind, pte) = self.next_l4()?;
        let frame = pte.frame().unwrap();
        let page = Page::from_page_table_indices(ind, self.l3?.0, self.l2?.0, self.l1?.0);
        Some((page, frame))
    }
}
