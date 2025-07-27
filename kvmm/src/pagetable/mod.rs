use cake::{Owned, info};
use x86_64::{
    VirtAddr,
    structures::paging::{PageTable, PageTableFlags, PageTableIndex},
};

use crate::{KernelPage, KernelPhysFrame, pagetable::page_layout::PageLayout};

mod entry;
mod page_layout;

pub type PageTablePath = (
    PageTableIndex,
    Option<PageTableIndex>,
    Option<PageTableIndex>,
);

/// A builder for creating a pagetable layout.
pub struct PagetableBuilder<T: Iterator<Item = (KernelPage, KernelPhysFrame)>> {
    alloc: Option<T>,
    pml4: Option<(Owned<PageTable>, KernelPhysFrame)>,
    layout: Option<Owned<PageLayout>>,
}

impl<T: Iterator<Item = (KernelPage, KernelPhysFrame)>> PagetableBuilder<T> {
    /// Creates a new pagetable builder.
    pub fn new(mut alloc: T) -> Self {
        let (pml4, frame) = alloc.next().expect("No pages provided");
        let pml4 = pml4.start_address().as_mut_ptr::<PageTable>();
        info!("Creating pagetable at {:#x}", pml4 as u64);
        unsafe {
            pml4.write_bytes(0, 1);
        }
        let pml4 = unsafe { Owned::new(pml4) };

        let (page, _) = alloc.next().expect("No pages provided");
        let layout = unsafe { PageLayout::create_in_page(page) };
        PagetableBuilder {
            alloc: Some(alloc),
            pml4: Some((pml4, frame)),
            layout: Some(layout),
        }
    }

    /// Returns the next page from the allocator.
    pub fn next_page(&mut self) -> (KernelPage, KernelPhysFrame) {
        self.alloc
            .as_mut()
            .expect("reclaimed")
            .next()
            .expect("No more pages available")
    }

    fn layout(&self) -> &PageLayout {
        self.layout.as_ref().expect("reclaimed")
    }

    fn layout_mut(&mut self) -> &mut PageLayout {
        self.layout.as_mut().expect("reclaimed")
    }

    fn push_pagetable(
        &mut self,
        pagetable: Owned<PageTable>,
        path: PageTablePath,
    ) -> &mut PageTable {
        if self.layout_mut().has_cap(1) {
            // We just checked that the layout has space, so unless something is horribly wrong, this should never fail.
            return self.layout_mut().push(pagetable, path);
        }

        let (page, _) = self.next_page();
        self.layout_mut().extend(page);

        self.layout_mut().push(pagetable, path)
    }

    fn create_pagetable(&mut self) -> (KernelPhysFrame, Owned<PageTable>) {
        let (page, frame) = self.next_page();
        let pagetable = page.start_address().as_mut_ptr::<PageTable>();
        unsafe {
            pagetable.write_bytes(0, 1);
        }

        (frame, unsafe { Owned::new(pagetable) })
    }

    fn get_or_create_l3(&mut self, pml4_index: PageTableIndex) -> &mut PageTable {
        let path = (pml4_index, None, None);

        if let Some(entry) = self.layout_mut().index_of(path) {
            return self.layout_mut()[entry].pagetable_mut();
        }

        let (paddr, pagetable) = self.create_pagetable();
        self.pml4.as_mut().expect("reclaimed").0[pml4_index]
            .set_frame(paddr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

        self.push_pagetable(pagetable, path)
    }

    fn get_or_create_l2(
        &mut self,
        pml4_index: PageTableIndex,
        pml3_index: PageTableIndex,
    ) -> &mut PageTable {
        let path = (pml4_index, Some(pml3_index), None);

        if let Some(entry) = self.layout_mut().index_of(path) {
            return self.layout_mut()[entry].pagetable_mut();
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

        if let Some(entry) = self.layout_mut().index_of(path) {
            return self.layout_mut()[entry].pagetable_mut();
        }

        let (paddr, pagetable) = self.create_pagetable();
        self.get_or_create_l2(pml4_index, pml3_index)[pml2_index]
            .set_frame(paddr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

        self.push_pagetable(pagetable, path)
    }

    /// Maps a page to a frame.
    pub fn map_page(&mut self, page: KernelPage, frame: KernelPhysFrame, flags: PageTableFlags) {
        let pagetable = self.get_or_create_l1(page.p4_index(), page.p3_index(), page.p2_index());
        let pte_index = page.p1_index();
        pagetable[pte_index].set_frame(frame, flags);
    }

    /// Maps a range of pages to frames. Will panic if len(pages) != len(frames).
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

    fn release(mut self, dtor: fn(KernelPage)) {
        unsafe { self.layout_mut().reclaim(dtor) };
        dtor(
            KernelPage::from_start_address(VirtAddr::from_ptr(unsafe {
                self.layout.take().unwrap().into_raw()
            }))
            .expect("unaligned"),
        );
    }

    pub fn frame(&self, page: KernelPage) -> Option<KernelPhysFrame> {
        let pml4_index = page.p4_index();
        let pml3_index = page.p3_index();
        let pml2_index = page.p2_index();
        let pte_index = page.p1_index();

        if let Some(entry) =
            self.layout()
                .index_of((pml4_index, Some(pml3_index), Some(pml2_index)))
        {
            let pagetable = self.layout()[entry].pagetable();
            return Some(pagetable[pte_index].frame().ok()?);
        }

        None
    }

    /// Build the pagetable and release the resources.
    /// Returns the pagetable, the frame it is mapped to, and the given iterator.
    ///
    /// This function only releases the memory used to store the pagetable layout, and will not run if `dtor` is `None`.
    pub fn build_and_release(
        mut self,
        dtor: Option<fn(KernelPage)>,
    ) -> (Owned<PageTable>, KernelPhysFrame, T) {
        let t = self.alloc.take().expect("reclaimed");
        let (pml4, frame) = self.pml4.take().expect("reclaimed");
        if let Some(dtor) = dtor {
            self.release(dtor);
        }
        (pml4, frame, t)
    }

    pub fn iterator(&mut self) -> &mut T {
        self.alloc.as_mut().expect("reclaimed")
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
mod tests {
    use super::*;
    use crate::test_util::DummyPageAllocator;

    #[test]
    fn test_pagetable_builder_new() {
        let mut alloc = DummyPageAllocator::new();
        println!(
            "{:p}",
            alloc
                .next()
                .expect("No pages provided")
                .0
                .start_address()
                .as_mut_ptr::<PageTable>()
        );

        let builder = PagetableBuilder::new(alloc.by_ref());

        let (pml4, frame, _) = builder.build_and_release(None);

        for i in 0..512 {
            assert!(pml4[i].is_unused());
        }

        let ptr = unsafe { pml4.into_raw() };
        for (page, addr) in alloc.used_pages() {
            if frame == *addr {
                assert_eq!(page.start_address().as_u64(), ptr as u64);
                return;
            }
        }
    }
}
