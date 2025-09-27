use alloc::collections::BTreeMap;
use log::debug;
use x86_64::structures::paging::{
    page_table::PageTableEntry, FrameAllocator, Mapper, Page, PageTable, PageTableFlags,
    PageTableIndex,
};

use crate::memory::paging::{
    phys::FRAME_ALLOCATOR, KernelPage, KernelPhysFrame, KERNEL_PAGE_TABLE,
};

pub struct PageTableBuilder<'a, T>
where
    T: Iterator<Item = KernelPage>,
{
    root: KernelPhysFrame,
    pml4: &'a mut PageTable,
    curr_page_range: T,
    /// Maps a tuple of (PML4 index, PDPT index, PD index) to a tuple of (frame, PageTable)
    /// This is weird, but keep in mind that as soon as this pagetable is loaded, it will be recursively mapped to the PML4 entry 510.
    // TODO: There is a way to optimize this by using the fact that we discard the lower 12 bits of the address.
    // It can be compacted to a single u64 key, which then can be formatted in a way to appease the b-tree gods.
    pub pagetables: BTreeMap<
        (
            PageTableIndex,
            Option<PageTableIndex>,
            Option<PageTableIndex>,
        ),
        (KernelPhysFrame, &'a mut PageTable),
    >,
}

impl<'a, T> PageTableBuilder<'a, T>
where
    T: Iterator<Item = KernelPage>,
{
    const RECURSIVE_ENTRY: usize = 509;
    const RECURSIVE_ENTRY_INDEX: PageTableIndex = PageTableIndex::new(Self::RECURSIVE_ENTRY as u16);

    pub fn new(mut page_range: T) -> Self {
        let mut pgtbl = KERNEL_PAGE_TABLE.write();
        let mut alc = FRAME_ALLOCATOR.get();
        let root_frame = alc
            .allocate_frame()
            .expect("Unable to allocate root frame for page table");
        let pml4 = page_range.next().expect("out of pages");
        unsafe {
            pgtbl
                .map_to(
                    pml4,
                    root_frame,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                    &mut *alc,
                )
                .expect("Failed to map PML4 page to root frame")
                .flush();
        };

        let ptr = pml4.start_address().as_u64() as *mut PageTable;

        unsafe {
            ptr.write(PageTable::new());
        }

        let pml4 = unsafe { &mut *(pml4.start_address().as_u64() as *mut PageTable) };

        PageTableBuilder {
            root: root_frame,
            pml4,
            curr_page_range: page_range,
            pagetables: BTreeMap::new(),
        }
        .make_recursive()
    }

    fn make_recursive(self) -> Self {
        let mut recursive_entry = PageTableEntry::new();
        recursive_entry.set_frame(
            self.root,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );

        self.pml4[Self::RECURSIVE_ENTRY] = recursive_entry;
        self
    }

    fn create_pagetable(&mut self) -> (KernelPhysFrame, &'a mut PageTable) {
        let mut alc = FRAME_ALLOCATOR.get();
        let mut offset_page_table = KERNEL_PAGE_TABLE.write();
        let pagetable_frame = alc
            .allocate_frame()
            .expect("Unable to allocate root frame for page table");
        let pagetable_page = self.curr_page_range.next().expect("out of pages");
        unsafe {
            offset_page_table
                .map_to(
                    pagetable_page,
                    pagetable_frame,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                    &mut *alc,
                )
                .expect("Failed to map pagetable page to frame")
                .flush();
        };

        let ptr = pagetable_page.start_address().as_u64() as *mut PageTable;

        unsafe {
            ptr.write(PageTable::new());
        }

        let pagetable =
            unsafe { &mut *(pagetable_page.start_address().as_u64() as *mut PageTable) };

        (pagetable_frame, pagetable)
    }

    fn get_or_create_l3(&mut self, pml4_index: PageTableIndex) -> &mut PageTable {
        let key = (pml4_index, None, None);

        if !self.pagetables.contains_key(&key) {
            debug!("Creating L3 pagetable for PML4 index {:?}", pml4_index);
            let (frame, pagetable) = self.create_pagetable();
            self.pagetables.insert(key, (frame, pagetable));
            let entry = new_pte(frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            self.pml4[pml4_index] = entry;
        }

        return self.pagetables.get_mut(&key).unwrap().1;
    }

    fn get_or_create_l2(
        &mut self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
    ) -> &mut PageTable {
        let key = (pml4_index, Some(pdpt_index), None);

        if !self.pagetables.contains_key(&key) {
            debug!(
                "Creating L2 pagetable for PML4 index {:?} and PDPT index {:?}",
                pml4_index, pdpt_index
            );
            let (frame, pagetable) = self.create_pagetable();
            self.pagetables.insert(key, (frame, pagetable));
            let entry = new_pte(frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            self.get_or_create_l3(pml4_index)[pdpt_index] = entry;
        }

        return self.pagetables.get_mut(&key).unwrap().1;
    }

    fn get_or_create_l1(
        &mut self,
        pml4_index: PageTableIndex,
        pdpt_index: PageTableIndex,
        pd_index: PageTableIndex,
    ) -> &mut PageTable {
        let key = (pml4_index, Some(pdpt_index), Some(pd_index));

        if !self.pagetables.contains_key(&key) {
            debug!(
                "Creating L1 pagetable for PML4 index {:?}, PDPT index {:?}, and PD index {:?}",
                pml4_index, pdpt_index, pd_index
            );
            let (frame, pagetable) = self.create_pagetable();
            self.pagetables.insert(key, (frame, pagetable));
            let entry = new_pte(frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            self.get_or_create_l2(pml4_index, pdpt_index)[pd_index] = entry;
        }

        return self.pagetables.get_mut(&key).unwrap().1;
    }

    pub fn map_page(&mut self, page: KernelPage, frame: KernelPhysFrame, flags: PageTableFlags) {
        let pagetable = self.get_or_create_l1(page.p4_index(), page.p3_index(), page.p2_index());
        let pte_index = page.p1_index();
        let entry = new_pte(frame, flags);
        pagetable[pte_index] = entry;
    }

    pub fn map_range<P, F>(&mut self, pages: &mut P, frames: &mut F, flags: PageTableFlags)
    where
        P: Iterator<Item = KernelPage>,
        F: Iterator<Item = KernelPhysFrame>,
    {
        for (i, page) in pages.enumerate() {
            if let Some(frame) = frames.next() {
                self.map_page(page, frame, flags);
            } else {
                // TODO: Panic is probably the right thing to do here, but we could also do a try_map_range
                // methods to return a Result instead.
                panic!("Not enough frames to map the range: offset {} pages", i);
            }
        }
    }

    pub fn build(self) -> (KernelPhysFrame, KernelPage) {
        (
            self.root,
            Page::from_page_table_indices(
                Self::RECURSIVE_ENTRY_INDEX,
                Self::RECURSIVE_ENTRY_INDEX,
                Self::RECURSIVE_ENTRY_INDEX,
                Self::RECURSIVE_ENTRY_INDEX,
            ),
        )
    }
}

fn new_pte(frame: KernelPhysFrame, flags: PageTableFlags) -> PageTableEntry {
    let mut entry = PageTableEntry::new();
    entry.set_frame(frame, flags);
    entry
}
