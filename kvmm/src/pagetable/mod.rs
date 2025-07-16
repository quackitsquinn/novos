use core::{
    mem::{MaybeUninit, transmute},
    ops::{Index, IndexMut},
};

use cake::Owned;
use x86_64::{
    VirtAddr,
    structures::paging::{Page, PageTable, PageTableFlags, PageTableIndex, Size4KiB},
};

use crate::{
    KernelPage, KernelPhysFrame,
    pagetable::{entry::Entry, page_layout::PageLayout},
};

mod entry;
mod page_layout;

pub type PageTablePath = (
    PageTableIndex,
    Option<PageTableIndex>,
    Option<PageTableIndex>,
);

/// A builder for creating a pagetable layout.
pub struct PagetableBuilder<T: Iterator<Item = (KernelPage, KernelPhysFrame)>> {
    alloc: T,
    pml4: (Owned<PageTable>, KernelPhysFrame),
    layout: Owned<PageLayout>,
}

impl<T: Iterator<Item = (KernelPage, KernelPhysFrame)>> PagetableBuilder<T> {
    /// Creates a new pagetable builder.
    pub fn new(mut alloc: T) -> Self {
        let (pml4, frame) = alloc.next().expect("No pages provided");
        let pml4 = pml4.start_address().as_mut_ptr::<PageTable>();
        unsafe {
            pml4.write_bytes(0, 4096);
        }
        let pml4 = unsafe { Owned::new(pml4) };

        let (page, _) = alloc.next().expect("No pages provided");
        let layout = unsafe { PageLayout::create_in_page(page) };
        PagetableBuilder {
            alloc,
            pml4: (pml4, frame),
            layout,
        }
    }

    fn next_page(&mut self) -> (KernelPage, KernelPhysFrame) {
        self.alloc.next().expect("No more pages available")
    }

    fn push_pagetable(
        &mut self,
        pagetable: Owned<PageTable>,
        path: PageTablePath,
    ) -> &mut PageTable {
        if self.layout.has_cap(1) {
            // We just checked that the layout has space, so unless something is horribly wrong, this should never fail.
            return self.layout.push(pagetable, path);
        }

        let (page, _) = self.next_page();
        self.layout.extend(page);

        self.layout.push(pagetable, path)
    }

    fn create_pagetable(&mut self) -> (KernelPhysFrame, Owned<PageTable>) {
        let (page, frame) = self.next_page();
        let pagetable = page.start_address().as_mut_ptr::<PageTable>();
        unsafe {
            pagetable.write_bytes(0, 4096);
        }

        (frame, unsafe { Owned::new(pagetable) })
    }

    fn get_or_create_l3(&mut self, pml4_index: PageTableIndex) -> &mut PageTable {
        let path = (pml4_index, None, None);

        if let Some(entry) = self.layout.index_of(path) {
            return self.layout[entry].pagetable();
        }

        let (paddr, pagetable) = self.create_pagetable();
        self.pml4.0[pml4_index]
            .set_frame(paddr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

        self.push_pagetable(pagetable, path)
    }

    fn get_or_create_l2(
        &mut self,
        pml4_index: PageTableIndex,
        pml3_index: PageTableIndex,
    ) -> &mut PageTable {
        let path = (pml4_index, Some(pml3_index), None);

        if let Some(entry) = self.layout.index_of(path) {
            return self.layout[entry].pagetable();
        }

        let (paddr, pagetable) = self.create_pagetable();
        self.get_or_create_l3(pml4_index)[pml3_index]
            .set_frame(paddr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

        self.push_pagetable(pagetable, path)
    }

    fn get_or_create_l1(
        &mut self,
        pml4_index: PageTableIndex,
        pml3_index: PageTableIndex,
        pml2_index: PageTableIndex,
    ) -> &mut PageTable {
        let path = (pml4_index, Some(pml3_index), Some(pml2_index));

        if let Some(entry) = self.layout.index_of(path) {
            return self.layout[entry].pagetable();
        }

        let (paddr, pagetable) = self.create_pagetable();
        self.get_or_create_l2(pml4_index, pml3_index)[pml2_index]
            .set_frame(paddr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

        self.push_pagetable(pagetable, path)
    }

    pub fn map_page(&mut self, page: KernelPage, frame: KernelPhysFrame, flags: PageTableFlags) {
        let pagetable = self.get_or_create_l1(page.p4_index(), page.p3_index(), page.p2_index());
        let pte_index = page.p1_index();

        pagetable[pte_index].set_frame(frame, flags);
    }

    pub fn map_range<P, F>(&mut self, pages: &mut P, frames: &mut F, flags: PageTableFlags)
    where
        P: Iterator<Item = KernelPage>,
        F: Iterator<Item = KernelPhysFrame>,
    {
        for page in pages {
            if let Some(frame) = frames.next() {
                self.map_page(page, frame, flags);
            } else {
                // TODO: Panic is probably the right thing to do here, but we could also do a try_map_range
                // methods to return a Result instead.
                panic!("Not enough frames to map the range");
            }
        }
    }
}
