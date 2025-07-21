use kvmm::KernelPage;
use x86_64::structures::paging::{Mapper, PageTableFlags, page::PageRange};

use crate::mem::{MAPPER, PAGETABLE};

pub struct MappedPageIterator {
    iter: PageRange,
}

impl MappedPageIterator {
    pub fn new(start: KernelPage, end: KernelPage) -> Self {
        MappedPageIterator {
            iter: KernelPage::range(start, end),
        }
    }
}

impl Iterator for MappedPageIterator {
    type Item = KernelPage;

    fn next(&mut self) -> Option<Self::Item> {
        let mut mapper = MAPPER.get().expect("uninit").lock();
        let mut table = PAGETABLE.get().expect("uninit").lock();
        let page = self.iter.next()?;
        let frame = mapper.next_frame()?;
        unsafe {
            table.map_to(
                page,
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                &mut *mapper,
            )
        }
        .expect("Failed to map page")
        .flush();
        Some(page)
    }
}
